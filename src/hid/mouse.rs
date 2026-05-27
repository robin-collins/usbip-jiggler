use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub type MouseReport = [u8; 3];

fn random_displacement() -> i8 {
    let mut rng = rand::thread_rng();
    // Pick from [−20,−1] ∪ [1,20]
    let mag = rng.gen_range(1i8..=20i8);
    if rng.gen_bool(0.5) { mag } else { -mag }
}

pub async fn jiggle_task(tx: mpsc::Sender<MouseReport>) {
    loop {
        sleep(Duration::from_secs(30)).await;

        let dx = random_displacement();
        let dy = random_displacement();

        let tick1: MouseReport = [0, dx as u8, dy as u8];
        let tick2: MouseReport = [0, (-dx) as u8, (-dy) as u8];

        let _ = tx.try_send(tick1);
        let _ = tx.try_send(tick2);
    }
}
