use postgres::NoTls;
use postgres::{Client, Statement};
use rust_decimal::Decimal;

use std::env;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

pub type Failable<T> = Result<T, Box<dyn Error>>;

fn connect() -> Result<Client, postgres::Error> {
    let db_user = env::var("PSQL_USER").unwrap();
    let db_host = env::var("PSQL_HOST").unwrap();
    let db_pass = env::var("PSQL_PASS").unwrap();
    let db_name = env::var("PSQL_DB").unwrap();
    let conf = format!("user={db_user} password={db_pass} dbname={db_name} hostaddr={db_host}");

    Client::connect(&conf, NoTls)
}

pub struct StatInstant {
    op_read: Decimal,
    ms_read: f64,
    op_write: Decimal,
    ms_write: f64,
}

pub struct CumulativeStat {
    pub us_read: f64,
    pub us_write: f64,
}

impl std::ops::Sub<StatInstant> for StatInstant {
    type Output = CumulativeStat;

    fn sub(self, _rhs: StatInstant) -> CumulativeStat {
        let cnt_read: i64 = (self.op_read - _rhs.op_read).try_into().unwrap();
        let cnt_write: i64 = (self.op_write - _rhs.op_write).try_into().unwrap();
        let ms_read = self.ms_read - _rhs.ms_read;
        let ms_write = self.ms_write - _rhs.ms_write;
        CumulativeStat {
            us_read: ms_read * 1000. / (cnt_read as f64),
            us_write: ms_write * 1000. / (cnt_write as f64),
        }
    }
}

pub struct StatObserver {
    client: Option<Client>,
    pstmt: Option<Statement>,
}

impl StatObserver {
    pub fn new() -> Failable<Self> {
        let mut client = connect()?;
        let pg_major: usize = env::var("PG_MAJOR").unwrap().parse()?;

        if pg_major >= 16 {
            let pstmt = client.prepare(
                "SELECT
                    sum(reads) as op_read,
                    sum(read_time) as ms_read,
                    sum(writes) as op_write,
                    sum(write_time) as ms_write
                    FROM pg_stat_io;",
            )?;
            println!("PG_MAJOR is {pg_major}. Enabling IO stat.");
            Ok(Self {
                client: Some(client),
                pstmt: Some(pstmt),
            })
        } else {
            println!("PG_MAJOR is {pg_major}. Disabling IO stat.");
            Ok(Self {
                client: None,
                pstmt: None,
            })
        }
    }

    pub fn valid(&self) -> bool {
        self.client.is_some() && self.pstmt.is_some()
    }

    pub fn stat(&mut self) -> Failable<StatInstant> {
        match (&mut self.client, &self.pstmt) {
            (Some(client), Some(pstmt)) => {
                let res = client.query_one(pstmt, &[])?;
                let op_read: Decimal = res.try_get("op_read")?;
                let ms_read: f64 = res.try_get("ms_read")?;
                let op_write: Decimal = res.try_get("op_write")?;
                let ms_write: f64 = res.try_get("ms_write")?;
                Ok(StatInstant {
                    op_read,
                    ms_read,
                    op_write,
                    ms_write,
                })
            }
            _ => Ok(StatInstant {
                op_read: Decimal::ZERO,
                ms_read: 0.,
                op_write: Decimal::ZERO,
                ms_write: 0.,
            }),
        }
    }
}

pub struct FunctionTimeline {
    pub start: Instant,
    pub end: Instant,
    pub name: String,
}

impl FunctionTimeline {
    pub fn new(name: &str) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            end: now,
            name: name.to_string(),
        }
    }

    pub fn end(&mut self) {
        self.end = Instant::now()
    }

    pub fn fmt_csv(&self, init_point: &Instant) -> String {
        let true_start = self.start - *init_point;
        let true_end = self.end - *init_point;
        format!(
            "{},{},{}\n",
            self.name,
            true_start.as_millis(),
            true_end.as_millis(),
        )
    }

    pub fn len_ms(&self) -> u128 {
        let dur = self.end - self.start;
        dur.as_millis()
    }
}

impl Drop for FunctionTimeline {
    fn drop(&mut self) {
        assert_ne!(
            self.start, self.end,
            "Unused FT object dropped: {}",
            self.name
        );
    }
}

enum SendQuery {
    Query(String, String),
    SeqQuery(String, Vec<String>),
    Exit,
}

enum RecvQuery {
    Sync,
}

struct GeneralStream {
    tx: Sender<SendQuery>,
    rx: Receiver<RecvQuery>,
    handle: JoinHandle<Vec<FunctionTimeline>>,
}

struct GeneralStreamGuard {
    stream: GeneralStream,
}

impl GeneralStreamGuard {
    fn sync(self) -> Failable<GeneralStream> {
        self.stream.sync()?;
        Ok(self.stream)
    }
}

impl GeneralStream {
    fn start() -> Failable<Self> {
        let (tx, rx_) = channel::<SendQuery>();
        let (tx_, rx) = channel::<RecvQuery>();

        let cl = connect()?;

        let handle = thread::spawn(move || {
            let mut timelines: Vec<FunctionTimeline> = Vec::new();
            let (tx, rx) = (tx_, rx_);
            let mut client = cl;

            while let Ok(sig) = rx.recv() {
                match sig {
                    SendQuery::Query(name, query) => {
                        let mut tl = FunctionTimeline::new(&name);
                        client.simple_query(&query).unwrap();
                        tl.end();
                        timelines.push(tl);
                        tx.send(RecvQuery::Sync).unwrap();
                    }
                    SendQuery::SeqQuery(name, seqquery) => {
                        for (i, query) in seqquery.iter().enumerate() {
                            let name = format!("{name}_{i}");
                            let mut tl = FunctionTimeline::new(&name);
                            client.simple_query(&query).unwrap();
                            tl.end();
                            timelines.push(tl);
                        }
                        tx.send(RecvQuery::Sync).unwrap();
                    }
                    SendQuery::Exit => break,
                }
            }
            timelines
        });

        Ok(Self { tx, rx, handle })
    }

    fn send_query(self, name: String, query: String) -> Failable<GeneralStreamGuard> {
        self.tx.send(SendQuery::Query(name, query))?;
        Ok(GeneralStreamGuard { stream: self })
    }

    fn send_seqquery(self, name: String, seqquery: Vec<String>) -> Failable<GeneralStreamGuard> {
        self.tx.send(SendQuery::SeqQuery(name, seqquery))?;
        Ok(GeneralStreamGuard { stream: self })
    }

    fn sync(&self) -> Result<(), Box<dyn Error>> {
        match self.rx.recv()? {
            RecvQuery::Sync => Ok(()),
        }
    }

    fn join(self) -> Vec<FunctionTimeline> {
        self.tx.send(SendQuery::Exit).unwrap();
        self.handle.join().unwrap()
    }
}

pub struct RefreshStream {
    stream: GeneralStream,
    queries: Vec<(String, String)>,
    temp_rf2: Option<String>,
    ith: usize,
}
pub struct RefreshStreamGuard {
    _guard: GeneralStreamGuard,
    queries: Vec<(String, String)>,
    temp_rf2: Option<String>,
    ith: usize,
}

impl RefreshStreamGuard {
    pub fn sync(self) -> Failable<RefreshStream> {
        let stream = self._guard.sync()?;
        Ok(RefreshStream {
            stream,
            queries: self.queries,
            temp_rf2: self.temp_rf2,
            ith: self.ith,
        })
    }
}

impl RefreshStream {
    pub fn prepare(start_i: usize, end_i: usize) -> Result<Self, Box<dyn Error>> {
        let rf1_prefix = env::var("PATH_PREFIX_RF1").unwrap();
        let rf1_suffix = env::var("PATH_SUFFIX_RF1").unwrap();
        let rf2_prefix = env::var("PATH_PREFIX_RF2").unwrap();
        let rf2_suffix = env::var("PATH_SUFFIX_RF2").unwrap();

        let mut queries = Vec::new();
        for i in start_i..end_i {
            let path_rf1 = format!("{rf1_prefix}{i}{rf1_suffix}");
            let path_rf2 = format!("{rf2_prefix}{i}{rf2_suffix}");

            let qrf1 = std::fs::read_to_string(path_rf1)?;
            let qrf2 = std::fs::read_to_string(path_rf2)?;
            queries.push((qrf1, qrf2));
        }
        let stream = GeneralStream::start()?;
        Ok(Self {
            stream,
            queries,
            temp_rf2: None,
            ith: 1,
        })
    }

    pub fn run_rf1(mut self) -> Failable<RefreshStreamGuard> {
        let (qrf1, qrf2) = self.queries.pop().unwrap();
        assert!(self.temp_rf2.replace(qrf2).is_none());

        let name = format!("RF1_{}", self.ith);

        let guard = self.stream.send_query(name, qrf1)?;

        Ok(RefreshStreamGuard {
            _guard: guard,
            queries: self.queries,
            temp_rf2: self.temp_rf2,
            ith: self.ith,
        })
    }

    pub fn run_rf2(mut self) -> Failable<RefreshStreamGuard> {
        let qrf2 = self.temp_rf2.take().unwrap();

        let name = format!("RF2_{}", self.ith);
        self.ith += 1;

        let guard = self.stream.send_query(name, qrf2)?;

        Ok(RefreshStreamGuard {
            _guard: guard,
            queries: self.queries,
            temp_rf2: self.temp_rf2,
            ith: self.ith,
        })
    }

    pub fn join(self) -> Vec<FunctionTimeline> {
        self.stream.join()
    }
}

pub struct SeqStream {
    stream: GeneralStream,
    name: String,
    queries: Option<Vec<String>>,
}

pub struct SeqStreamGuard {
    _guard: GeneralStreamGuard,
    name: String,
    queries: Option<Vec<String>>,
}

impl SeqStreamGuard {
    pub fn sync(self) -> Failable<SeqStream> {
        let stream = self._guard.sync()?;
        Ok(SeqStream {
            stream,
            name: self.name,
            queries: self.queries,
        })
    }
}

impl SeqStream {
    pub fn prepare(name: &str, sql_file: &str) -> Result<Self, Box<dyn Error>> {
        let entire_query = std::fs::read_to_string(sql_file)?;

        let mut rqfs: Vec<String> = Vec::new();

        let mut vqry_tmp: Option<Vec<String>> = None;
        for rqry in entire_query.split(';') {
            let tqry = rqry.trim().to_string();

            if tqry.is_empty() {
                continue;
            }

            if vqry_tmp.is_none() {
                if tqry.starts_with("create view revenue") {
                    vqry_tmp = Some(vec![tqry.to_string()]);
                    continue;
                }
            } else {
                let trg = vqry_tmp.as_mut().unwrap();
                trg.push(tqry);

                if trg.len() == 3 {
                    let qry: String = vqry_tmp.take().unwrap().join(";");
                    rqfs.push(qry);
                }
                continue;
            }

            rqfs.push(tqry);
        }
        Ok(Self {
            stream: GeneralStream::start()?,
            name: name.to_string(),
            queries: Some(rqfs),
        })
    }

    pub fn prepare_power() -> Failable<Self> {
        let sql_file = env::var("PATH_QRY_POWER")?;
        let name = format!("PWRSTREAM");
        Self::prepare(&name, &sql_file)
    }

    pub fn prepare_throughput(i: usize) -> Failable<Self> {
        let prefix = env::var("PATH_PREFIX_QRY_THROUGHPUT")?;
        let suffix = env::var("PATH_SUFFIX_QRY_THROUGHPUT")?;

        let sql_file = format!("{prefix}{i}{suffix}");
        let name = format!("THRSTREAM{i}");
        Self::prepare(&name, &sql_file)
    }

    pub fn run(mut self) -> Failable<SeqStreamGuard> {
        let queries = self.queries.take().unwrap();

        let guard = self.stream.send_seqquery(self.name.clone(), queries)?;
        Ok(SeqStreamGuard {
            _guard: guard,
            name: self.name,
            queries: self.queries,
        })
    }

    pub fn join(self) -> Vec<FunctionTimeline> {
        self.stream.join()
    }
}
