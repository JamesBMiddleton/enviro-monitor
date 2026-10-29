import socket

HOST = "0.0.0.0"
PORT = 8080

def main():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((HOST, PORT))
        server.listen()
        print(f"Listening on {HOST}:{PORT}...")

        while True:
            conn, addr = server.accept()
            with conn:
                print(f"Connection from {addr[0]}, {addr[1]}")

                data = conn.recv(1024)
                if not data:
                    print("Client closed connection without sending data")
                    continue

                print(f"Recieved: {data.decode('utf-8', errors='replace').strip()}")

                print(f"Closing connection")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nShutting down")
