static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

#[embassy_executor::task]
async fn imu_task(mut mpu: Mpu6500) {
    mpu.init().await;
    defmt::info!("Calibrating, keep sensor still...");
    let bias = calibrate(&mut mpu, 1000).await;
    defmt::info!("Calibrating finished: {}", bias);

    let mut gyro_filter = GyroFilter::new(8000.0, 80.0);
    let mut ticker = Ticker::every(Duration::from_hz(8000));

    let mut timecounter: u32 = 0;
    let mut max_duration: Duration = Duration::from_micros(0);
    let mut time_read_burst: Duration = Duration::from_micros(0);
    const TICK_BUDGET: Duration = Duration::from_hz(8000);

    loop {
        let start = Instant::now();

        let raw = mpu.read_burst().await;

        if let Some(raw) = raw {
            let mut imu = bias.apply(raw.convert());
            time_read_burst = start.elapsed();

            imu.gyro = gyro_filter.apply(imu.gyro);

            IMU_SIGNAL.signal(imu);
        }
        let end = start.elapsed();

        if timecounter.is_multiple_of(400) {
            log::info!(
                "burst_duration:{} total_duration:{} max_duration:{}",
                time_read_burst,
                end,
                max_duration
            );
        }

        if end > TICK_BUDGET {
            defmt::warn!(
                "IMU overrun! Loop duurde {} µs (budget is {} µs)",
                end.as_micros(),
                TICK_BUDGET.as_micros()
            );
        }

        if end > max_duration {
            max_duration = end;
        }
        timecounter += 1;

        ticker.next().await;
    }
}
