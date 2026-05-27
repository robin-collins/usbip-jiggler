use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[repr(C, packed)]
pub struct MouseReport {
    pub buttons: u8, // bit0=left, bit1=right, bit2=middle
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
}

impl MouseReport {
    pub fn zero() -> Self {
        Self { buttons: 0, x: 0, y: 0, wheel: 0 }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [self.buttons, self.x as u8, self.y as u8, self.wheel as u8]
    }
}

fn random_displacement() -> i8 {
    let mut rng = rand::thread_rng();
    let mag = rng.gen_range(1i8..=20i8);
    if rng.gen_bool(0.5) { mag } else { -mag }
}

pub async fn jiggle_task(tx: mpsc::Sender<[u8; 4]>) {
    loop {
        sleep(Duration::from_secs(30)).await;

        let dx = random_displacement();
        let dy = random_displacement();

        let tick1 = MouseReport { buttons: 0, x: dx, y: dy, wheel: 0 }.to_bytes();
        let tick2 = MouseReport { buttons: 0, x: -dx, y: -dy, wheel: 0 }.to_bytes();

        let _ = tx.try_send(tick1);
        let _ = tx.try_send(tick2);
    }
}
