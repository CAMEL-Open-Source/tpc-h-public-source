use dotenv::dotenv;
use std::{env, time::Instant};

mod stream;
use stream::{CumulativeStat, Failable, FunctionTimeline, RefreshStream, SeqStream, StatObserver};

struct TotalTimeline {
    metric: f64,
    start: Instant,
    timelines: Vec<FunctionTimeline>,
}

impl TotalTimeline {
    fn print(&self) {
        for tl in &self.timelines {
            print!("{}", tl.fmt_csv(&self.start));
        }
    }
}

fn power_test(scale_factor: usize) -> Failable<TotalTimeline> {
    let mut rs = RefreshStream::prepare(1, 2)?;
    let mut ps = SeqStream::prepare_power()?;

    let start = Instant::now();

    rs = rs.run_rf1()?.sync()?;
    ps = ps.run()?.sync()?;
    rs = rs.run_rf2()?.sync()?;

    let result_rs = rs.join();
    let result_ps = ps.join();

    assert_eq!(result_rs.len(), 2);
    assert_eq!(result_ps.len(), 22);

    // Accumulate timeline information
    let mut timelines = Vec::new();
    timelines.extend(result_rs);
    timelines.extend(result_ps);

    // Calculate TPC-H Power
    let tpch_power: f64 = {
        let mut acc: f64 = 1.0;
        for tl in &timelines {
            let v = tl.len_ms() as f64 / 1000.0;
            acc *= v;
        }
        acc = acc.powf(1.0 / 24.0);
        let upper = (3600 * scale_factor) as f64;
        let lower = acc;
        upper / lower
    };
    println!("Power@SF{scale_factor}={tpch_power}");

    Ok(TotalTimeline {
        metric: tpch_power,
        start,
        timelines,
    })
}

fn throughput_test(scale_factor: usize, num_stream: usize) -> Failable<TotalTimeline> {
    let mut rs = RefreshStream::prepare(2, 2 + num_stream)?;
    let mut tss = Vec::new();

    for i in 0..num_stream {
        tss.push(SeqStream::prepare_throughput(1 + i)?);
    }

    let start = Instant::now();

    let mut _tss = Vec::new();
    for ts in tss.into_iter() {
        _tss.push(ts.run()?);
    }

    for _ in 0..num_stream {
        rs = rs.run_rf1()?.sync()?;
        rs = rs.run_rf2()?.sync()?;
    }

    let mut tss = Vec::new();
    for ts in _tss.into_iter() {
        tss.push(ts.sync()?);
    }

    // Accumulate timeline information
    let mut timelines = Vec::new();
    timelines.extend(rs.join());
    for ts in tss {
        timelines.extend(ts.join());
    }

    // Caculate TPC-H Throughput
    let tpch_throughput = {
        let min = timelines
            .iter()
            .map(|tl| tl.start)
            .reduce(|acc, s| acc.min(s))
            .unwrap();
        let max = timelines
            .iter()
            .map(|tl| tl.end)
            .reduce(|acc, s| acc.max(s))
            .unwrap();
        let upper = (num_stream * 22 * 3600) as f64;
        let lower = ((max - min).as_millis() * scale_factor as u128) as f64 / 1000.;
        upper / lower
    };
    println!("Throughput@SF{scale_factor}={tpch_throughput}");

    Ok(TotalTimeline {
        metric: tpch_throughput,
        start,
        timelines,
    })
}

fn main() -> Failable<()> {
    dotenv().ok();

    let num_stream: usize = {
        let ns = env::var("NUM_STREAM").unwrap();
        ns.parse().unwrap()
    };

    let scale_factor: usize = {
        let sf = env::var("SCALE_FACTOR").unwrap();
        sf.parse().unwrap()
    };

    let mut stat_observer = StatObserver::new()?;

    let stat_begin = stat_observer.stat()?;

    println!("SF={scale_factor} / NS={num_stream}");
    println!("Starting power test...");
    let result_pwr = power_test(scale_factor)?;
    println!("Starting throughput test...");
    let result_thp = throughput_test(scale_factor, num_stream)?;

    let stat_end = stat_observer.stat()?;

    if stat_observer.valid() {
        let CumulativeStat { us_read, us_write } = stat_end - stat_begin;
        println!("Average read latency(us): {us_read}");
        println!("Average write latency(us): {us_write}");
    }

    let pwr = result_pwr.metric;
    let thp = result_thp.metric;
    println!(
        "Composite Qph@SF{scale_factor}={}",
        (pwr * thp).sqrt()
    );

    println!("--- Debugging information start ---");
    println!("Power test timeline\nname,start,end");
    result_pwr.print();
    println!("Throughput test timeline\nname,start,end");
    result_thp.print();

    Ok(())
}
