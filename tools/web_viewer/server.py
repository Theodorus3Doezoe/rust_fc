import serial
import struct
import asyncio
import threading
import time
import json
import os
import math
from typing import Set
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel
import serial.tools.list_ports

app = FastAPI()

def quat_to_euler(w: float, x: float, y: float, z: float):
    roll = math.atan2(2.0 * (w * x + y * z), 1.0 - 2.0 * (x * x + y * y))
    pitch = math.asin(max(-1.0, min(1.0, 2.0 * (w * y - z * x))))
    yaw = math.atan2(2.0 * (w * z + x * y), 1.0 - 2.0 * (y * y + z * z))
    return (
        math.degrees(roll),
        math.degrees(pitch),
        math.degrees(yaw),
    )

# Global state
clients: Set[WebSocket] = set()
latest_data = {
    "accel_x": 0.0,
    "accel_y": 0.0,
    "accel_z": 0.0,
    "gyro_x": 0.0,
    "gyro_y": 0.0,
    "gyro_z": 0.0,
    "roll": 0.0,
    "pitch": 0.0,
    "yaw": 0.0,
    "qw": 1.0,
    "qx": 0.0,
    "qy": 0.0,
    "qz": 0.0,
}

serial_connection = None
serial_thread = None
serial_running = False
active_port = None

class ConnectRequest(BaseModel):
    port: str
    baudrate: int = 115200

def serial_reader(port_name: str, baudrate: int):
    global serial_connection, serial_running, latest_data, active_port
    try:
        ser = serial.Serial(port_name, baudrate, timeout=0.1)
        serial_connection = ser
        serial_running = True
        active_port = port_name
        print(f"Connected to serial port: {port_name}")
    except Exception as e:
        print(f"Failed to open serial port {port_name}: {e}")
        serial_running = False
        active_port = None
        return

    buffer = bytearray()
    while serial_running:
        try:
            if ser.in_waiting > 0:
                data = ser.read(ser.in_waiting)
                buffer.extend(data)
                
                while len(buffer) >= 4:
                    idx = buffer.find(b'\xaa\xbb')
                    if idx == -1:
                        if buffer[-1] == 0xAA:
                            buffer = buffer[-1:]
                        else:
                            buffer.clear()
                        break
                    
                    if idx > 0:
                        buffer = buffer[idx:]
                    
                    if len(buffer) < 4:
                        break
                    
                    payload_len = buffer[2]
                    total_packet_len = payload_len + 4
                    
                    if len(buffer) < total_packet_len:
                        break
                    
                    packet = buffer[:total_packet_len]
                    buffer = buffer[total_packet_len:]
                    
                    msg_type = packet[3]
                    payload = packet[4:total_packet_len-1]
                    checksum = packet[total_packet_len-1]
                    
                    calc_checksum = msg_type
                    for b in payload:
                        calc_checksum = (calc_checksum + b) & 0xFF
                        
                    if calc_checksum != checksum:
                        print("Checksum mismatch!")
                        continue
                    
                    if msg_type == 1 and len(payload) == 24: # ImuData
                        ax, ay, az, gx, gy, gz = struct.unpack('<ffffff', payload)
                        latest_data["accel_x"] = ax
                        latest_data["accel_y"] = ay
                        latest_data["accel_z"] = az
                        latest_data["gyro_x"] = gx
                        latest_data["gyro_y"] = gy
                        latest_data["gyro_z"] = gz
                    elif msg_type == 2 and len(payload) == 16: # VqfOrientation (quaternion)
                        qw, qx, qy, qz = struct.unpack('<ffff', payload)
                        roll, pitch, yaw = quat_to_euler(qw, qx, qy, qz)
                        latest_data["qy"] = qy
                        latest_data["qz"] = qz
                        latest_data["qx"] = qx
                        latest_data["qw"] = qw
                        latest_data["roll"] = roll
                        latest_data["pitch"] = pitch
                        latest_data["yaw"] = yaw
            else:
                time.sleep(0.002)
        except Exception as e:
            print(f"Error reading serial: {e}")
            break
            
    try:
        ser.close()
    except:
        pass
    serial_running = False
    active_port = None
    print("Serial connection closed.")

@app.get("/api/ports")
def get_ports():
    ports = []
    default_port = None
    
    for p in serial.tools.list_ports.comports():
        ports.append(p.device)
        if p.vid == 0xC0DE and p.pid == 0xCAFE:
            default_port = p.device
    
    defaults = ["/dev/ttyACM0", "/dev/ttyACM1", "/dev/ttyUSB0", "/dev/ttyUSB1"]
    for d in defaults:
        if os.path.exists(d) and d not in ports:
            ports.append(d)
            if default_port is None:
                default_port = d
    
    return {"ports": ports, "default": default_port}

@app.post("/api/connect")
def connect_serial(req: ConnectRequest):
    global serial_thread, serial_running
    if serial_running:
        return {"status": "error", "message": "Already connected"}
    
    serial_running = False
    serial_thread = threading.Thread(
        target=serial_reader,
        args=(req.port, req.baudrate),
        daemon=True
    )
    serial_thread.start()
    
    time.sleep(0.5)
    if serial_running:
        return {"status": "success", "port": req.port}
    else:
        return {"status": "error", "message": "Failed to connect to serial port"}

@app.post("/api/disconnect")
def disconnect_serial():
    global serial_running
    if not serial_running:
        return {"status": "error", "message": "Not connected"}
    serial_running = False
    return {"status": "success"}

@app.get("/api/status")
def get_status():
    return {
        "connected": serial_running,
        "port": active_port,
        "latest_data": latest_data
    }

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    clients.add(websocket)
    try:
        while True:
            # Keep connection alive with ping/pong
            await websocket.receive_text()
    except WebSocketDisconnect:
        pass
    finally:
        clients.remove(websocket)

async def broadcast_task():
    while True:
        if clients:
            message = json.dumps(latest_data)
            await asyncio.gather(
                *[client.send_text(message) for client in clients],
                return_exceptions=True
            )
        await asyncio.sleep(0.02)

@app.on_event("startup")
async def startup_event():
    asyncio.create_task(broadcast_task())

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
STATIC_DIR = os.path.join(BASE_DIR, "static")
app.mount("/", StaticFiles(directory=STATIC_DIR, html=True), name="static")
