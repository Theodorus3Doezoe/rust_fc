use super::frame::DshotTelemetry;
use core::sync::atomic::{AtomicU16, AtomicU32, AtomicUsize, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub struct EscLink {
    tx: Signal<CriticalSectionRawMutex, u16>,
    raw: AtomicU16,
    erpm: AtomicU32,
    erpm_period_us: AtomicU32,
}

impl EscLink {
    pub const fn new() -> Self {
        Self {
            tx: Signal::new(),
            raw: AtomicU16::new(u16::MAX),
            erpm: AtomicU32::new(u32::MAX),
            erpm_period_us: AtomicU32::new(u32::MAX),
        }
    }

    pub fn send(&self, frame: u16) {
        self.tx.signal(frame);
    }

    pub async fn wait(&self) -> u16 {
        self.tx.wait().await
    }

    pub fn publish(&self, raw: u16, erpm: u32, period: u32) {
        self.raw.store(raw, Ordering::Relaxed);
        self.erpm.store(erpm, Ordering::Relaxed);
        self.erpm_period_us.store(period, Ordering::Relaxed);
    }

    pub fn latest(&self) -> Option<DshotTelemetry> {
        let raw = self.raw.load(Ordering::Relaxed);
        let erpm = self.erpm.load(Ordering::Relaxed);
        let erpm_period_us = self.erpm_period_us.load(Ordering::Relaxed);

        if raw == u16::MAX || erpm == u32::MAX || erpm_period_us == u32::MAX {
            return None;
        }
        Some(DshotTelemetry {
            raw,
            erpm,
            erpm_period_us,
        })
    }
}

static ESC_LINKS: [EscLink; 4] = [
    EscLink::new(),
    EscLink::new(),
    EscLink::new(),
    EscLink::new(),
];

static LINK_CNT: AtomicUsize = AtomicUsize::new(0);

pub fn claim_link() -> Option<&'static EscLink> {
    let num = LINK_CNT.fetch_add(1, Ordering::Relaxed);
    let link = ESC_LINKS.get(num);

    if link.is_none() {
        defmt::warn!("Motor {} is out of bounds", num);
    }
    link
}

pub fn claimed_links() -> usize {
    LINK_CNT.load(Ordering::Relaxed)
}

pub fn link_at(k: usize) -> Option<&'static EscLink> {
    ESC_LINKS.get(k)
}
