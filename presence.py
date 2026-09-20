import time
import sys
from pypresence import Presence, DiscordNotFound

CLIENT_ID = "1516355651364716554"
ASSET_NAME = "wallpaperbetter_com_3840x2160_2_"

def main():
    print(f"Connecting to Discord RPC with Client ID: {CLIENT_ID}...")
    rpc = Presence(CLIENT_ID)
    
    try:
        rpc.connect()
        print("Connected to Discord Desktop client successfully!")
    except DiscordNotFound:
        print("Error: Could not find Discord Desktop client. Please ensure Discord is running on your machine.")
        sys.exit(1)
    except Exception as e:
        print(f"Connection Error: {e}")
        sys.exit(1)

    start_time = int(time.time())

    rpc.update(
        state="Active",
        details="Testing Themis Presence",
        large_image=ASSET_NAME,
        large_text="Themis App",
        start=start_time,
    )
    print(f"Rich Presence activated! Displaying 'Playing Themis' with asset '{ASSET_NAME}'.")
    print("Press Ctrl+C to stop.")

    try:
        while True:
            time.sleep(15)
    except KeyboardInterrupt:
        print("\nClosing Discord RPC connection...")
        rpc.close()
        print("Presence stopped.")

if __name__ == "__main__":
    main()
