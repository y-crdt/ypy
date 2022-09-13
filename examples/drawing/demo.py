import subprocess
from typing import List


def demo():
    """
    Spawns a server and two drawing clients.
    """
    processes: List[subprocess.Popen]  = []
    # Server
    processes.append(subprocess.Popen(["python", "server.py"]))

    # Clients
    for _ in range(2):
        processes.append(subprocess.Popen(["python", "draw.py"]))


    wait_until_done()

    for p in processes:
        p.kill()
    


def wait_until_done():
    print("waiting")
    while input("Enter 'q' to quit: ").lower() != 'q':
        continue



if __name__ == "__main__":
    demo()