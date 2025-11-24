import socket
import time

CHUNK = 40960


def stream_dbn(path: str, host: str = "127.0.0.1", port: int = 9001):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind((host, port))
        s.listen(1)

        while True:
            print("Waiting for connection...")
            conn, _ = s.accept()
            total_sent = 0

            start_time = time.time()  # ⬅️ start the timer

            try:
                print("Client connected")

                with open(path, "rb") as f:
                    while chunk := f.read(CHUNK):
                        total_sent += len(chunk)
                        print(
                            f"Sending {len(chunk)} bytes, total sent: {total_sent} bytes"
                        )
                        conn.sendall(chunk)
                        print(f"Sent {len(chunk)} bytes")
                        # optional throttling here
            finally:
                print("Closing connection")
                conn.close()

                elapsed = time.time() - start_time
                if elapsed > 0:
                    speed_bps = total_sent / elapsed
                    speed_mbps = speed_bps / (1024 * 1024)

                    print(
                        f"\n[STATS] Sent {total_sent:,} bytes "
                        f"in {elapsed:.3f} s "
                        f"({speed_bps:,.0f} B/s, {speed_mbps:.2f} MB/s)"
                    )


if __name__ == "__main__":
    stream_dbn("CLX5_mbo.dbn")
