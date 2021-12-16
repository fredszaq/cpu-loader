use std::sync::mpsc::{channel, RecvTimeoutError, Sender, TryRecvError};
use std::time::{Duration, Instant};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "cpu-loader", about = "Simple program loading a cpu")]
struct CliArgs {
    /// Number of threads to use
    #[structopt(long, default_value = "1")]
    threads: usize,

    /// Target load (in percent) for each thread
    #[structopt(long, default_value = "100")]
    load_percent: u8,

    /// Duration to run, will run indefinitely if not set
    #[structopt(long)]
    duration_seconds: Option<u64>,
}

pub struct CpuLoadThread {
    handle: Option<std::thread::JoinHandle<()>>,
    tx: Sender<()>,
}

impl CpuLoadThread {
    pub fn start_with_load_and_name<S: Into<String>>(
        load_percent: u8,
        thread_name: S,
    ) -> CpuLoadThread {
        assert!(load_percent <= 100);
        let (tx, rx) = channel();
        let handle = Some(
            std::thread::Builder::new()
                .name(thread_name.into())
                .spawn(move || 'outer: loop {
                    let start = Instant::now();
                    loop {
                        match rx.try_recv() {
                            Ok(()) | Err(TryRecvError::Disconnected) => break 'outer,
                            Err(TryRecvError::Empty) => {
                                if start.elapsed()
                                    > Duration::from_micros(100 * load_percent as u64)
                                {
                                    break;
                                }
                            }
                        }
                    }
                    if load_percent < 100 {
                        match rx
                            .recv_timeout(Duration::from_micros(10000 - 100 * load_percent as u64))
                        {
                            Ok(()) | Err(RecvTimeoutError::Disconnected) => break 'outer,
                            Err(RecvTimeoutError::Timeout) => {}
                        }
                    }
                })
                .unwrap(),
        );
        CpuLoadThread { handle, tx }
    }
}

impl Drop for CpuLoadThread {
    fn drop(&mut self) {
        self.tx.send(()).unwrap();
        self.handle.take().unwrap().join().unwrap()
    }
}

fn main() {
    let cli_args: CliArgs = StructOpt::from_args();

    let threads: Vec<_> = (0..cli_args.threads)
        .map(|i| {
            CpuLoadThread::start_with_load_and_name(cli_args.load_percent, format!("cpuload{}", i))
        })
        .collect();

    let wait_duration = cli_args
        .duration_seconds
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(u64::MAX));

    std::thread::sleep(wait_duration);
    drop(threads)
}
