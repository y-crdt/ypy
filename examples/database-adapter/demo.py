import subprocess
from time import sleep
from typing import List


def demo():
    """
    Spawns a server and two clients.
    """
    processes: List[subprocess.Popen]  = []
    py = get_python_command()
    # Server
    processes.append(subprocess.Popen([py, "server.py"]))
    sleep(1)
    # Clients
    for _ in range(2):
        processes.append(subprocess.Popen([py, "client.py"]))


    wait_until_done()

    for p in processes:
        p.kill()

def get_python_command() -> str:
    for name in ["python3", "python"]:
        command_exists = subprocess.Popen(["which", name]).wait() == 0
        if command_exists:
            return name
    raise Exception("No Python command found in shell.")
    


def wait_until_done():
    print("waiting")
    while input("Enter 'q' to quit: ").lower() != 'q':
        continue



if __name__ == "__main__":
    demo()