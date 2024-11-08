use std::{io::Error, num::NonZeroU32, path::PathBuf, thread::{self, JoinHandle}, time::Duration};
use governor::{Quota, RateLimiter};
use holani::mikey::uart::comlynx_cable_mutex::ComlynxCable;
use crate::debugger::{session::LynxSession, settings::Settings};

#[derive(Clone)]
pub struct AddCoreConfiguration {
    pub settings: Settings,
    pub cart_path: PathBuf,
}

pub struct CoreRunner {
    runner_thread: Option<JoinHandle<()>>,
    add_core_tx: Option<kanal::Sender<AddCoreConfiguration>>,
    add_core_done_rx: Option<kanal::Receiver<Result<(), Error>>>,
    show_tx: Option<kanal::Sender<egui::Context>>,
    show_done_rx: Option<kanal::Receiver<()>>,
}

impl Default for CoreRunner {
    fn default() -> Self {
        CoreRunner::new()
    }
}

impl Drop for CoreRunner {
    fn drop(&mut self) {
        if let Some(tx) = self.add_core_tx.take() {
            tx.close().unwrap();
            if let Some(handle) = self.runner_thread.take() {
                handle.join().unwrap();
            }
        }
    }
}

impl CoreRunner {
    pub fn new() -> Self {
        Self {
            runner_thread: None,
            add_core_tx: None,
            add_core_done_rx: None,
            show_tx: None,
            show_done_rx: None,
        }
    }

    pub fn add_core(&self, conf: &AddCoreConfiguration) -> Result<(), Error> {
        self.add_core_tx.as_ref().unwrap().send(conf.clone()).unwrap();
        self.add_core_done_rx.as_ref().unwrap().recv().unwrap()
    }

    pub fn show(&self, ctx: &egui::Context) {
        self.show_tx.as_ref().unwrap().send(ctx.clone()).unwrap();
        self.show_done_rx.as_ref().unwrap().recv().unwrap();
    }

    pub fn initialize_thread(&mut self) {
        let (add_core_tx, add_core_rx) = kanal::unbounded::<AddCoreConfiguration>();
        let (add_core_done_tx, add_core_done_rx) = kanal::unbounded::<Result<(), Error>>();
        let (show_tx, show_rx) = kanal::unbounded::<egui::Context>();
        let (show_done_tx, show_done_rx) = kanal::unbounded::<()>();

        self.runner_thread = Some(
            std::thread::Builder::new()
            .name("Core runner".to_string())
            .spawn(move || {
                const CRYSTAL_FREQUENCY: u32 = 16_000_000;
                const SLEEP: Duration = Duration::from_nanos(20);
                
                let lim = RateLimiter::direct(Quota::per_second(NonZeroU32::new(CRYSTAL_FREQUENCY).unwrap()));
                let mut sessions: Vec<LynxSession> = vec![];
                let comlynx: ComlynxCable = ComlynxCable::default();

                loop {
                    while lim.check().is_err() {
                        thread::sleep(SLEEP);
                    }

                    if add_core_rx.is_disconnected() {
                        return;
                    } else if let Ok(Some(conf)) = add_core_rx.try_recv() {
                        match LynxSession::new(sessions.len()+1, &comlynx, conf.cart_path, &conf.settings) {
                            Ok(s) => {
                                sessions.push(s);
                                add_core_done_tx.send(Ok(()))
                            },
                            Err(e) => add_core_done_tx.send(Err(e))
                        }.unwrap();
                    } else {
                        sessions.iter_mut().for_each(|s| s.step());

                        if let Ok(Some(ctx)) = show_rx.try_recv() {
                            sessions.iter_mut().for_each(|s| {
                                s.show(&ctx);
                                s.handle_inputs(&ctx);
                            });
                            ctx.request_repaint_after(Duration::from_secs(1/75));
                            show_done_tx.send(()).unwrap();
                        }
                    }
                }
            })
            .expect("Could not create the main core runner thread.")
        );
        self.add_core_tx = Some(add_core_tx);
        self.add_core_done_rx = Some(add_core_done_rx);
        self.show_tx = Some(show_tx);
        self.show_done_rx = Some(show_done_rx);
    }   
}
