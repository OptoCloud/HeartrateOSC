import time

from pythonosc import udp_client

client = udp_client.SimpleUDPClient("127.0.0.1", 9000)

while True:
    client.send_message("/avatar/parameters/isWristVisible", 1)
    client.send_message("/avatar/parameters/XSO Toggle", 1)
    time.sleep(1)