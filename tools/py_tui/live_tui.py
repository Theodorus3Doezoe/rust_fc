import argparse
import csv
import time
from datetime import datetime
import serial
import os


parser = argparse.ArgumentParser(description="Live IMU-data viewer over USB-serial")
parser.add_argument("--device", "-d", type=int, default=0)
parser.add_argument("--baud", "-b", type=int, default=115200)
parser.add_argument(
    "--rate", "-r", type=float, default=0.2, help="Terminal update-interval in seconden"
)
parser.add_argument(
    "--log", "-l", type=str, default=None, help="Pad naar CSV-logbestand (optioneel)"
)
args = parser.parse_args()

port = f"/dev/ttyACM{args.device}"
ser = serial.Serial(port, args.baud)

print(f"Verbonden met {port} @ {args.baud} baud\n")

log_file = None
csv_writer = None
if args.log:
    os.makedirs(os.path.dirname(args.log), exist_ok=True)
    log_file = open(args.log, "w", newline="")
    csv_writer = csv.writer(log_file)
    print(f"Loggen naar {args.log}\n")

last_update = 0.0
header_written = False

while True:
    line = ser.readline().decode("utf-8", errors="ignore").strip()
    if not line:
        continue

    # parse "label:waarde" paren
    parts = line.split()
    parsed = {}
    for part in parts:
        if ":" in part:
            label, value = part.split(":", 1)
            try:
                parsed[label] = float(value)
            except ValueError:
                continue

    if not parsed:
        continue

    # elke regel loggen naar CSV, ongeacht terminal-update-rate
    if csv_writer:
        if not header_written:
            csv_writer.writerow(["timestamp"] + list(parsed.keys()))
            header_written = True
        csv_writer.writerow([datetime.now().isoformat()] + list(parsed.values()))
        log_file.flush()

    # terminal alleen met vertraagde update-rate
    now = time.time()
    if now - last_update < args.rate:
        continue
    last_update = now

    display = " ".join(f"{k}:{v:.2f}" for k, v in parsed.items())
    print(f"\r{display}" + " " * 20, end="", flush=True)
