#!/usr/bin/env python3
"""
WebSocket real-time events example for slskr Python client
"""

import asyncio
from slskr import SlskrClient


async def handle_message(event):
    """Handle incoming message event"""
    print(f"Message from {event.get('username')}: {event.get('content')[:50]}...")


async def handle_search_update(event):
    """Handle search update event"""
    print(f"Search update: {event.get('query')} - {event.get('result_count')} results")


async def handle_transfer_update(event):
    """Handle transfer status update"""
    print(f"Transfer {event.get('id')}: {event.get('status')} - {event.get('progress')}%")


async def handle_connection_change(connected):
    """Handle connection state changes"""
    if connected:
        print("✓ WebSocket connected")
    else:
        print("✗ WebSocket disconnected")


async def handle_error(error):
    """Handle WebSocket errors"""
    print(f"Error: {error}")


async def main():
    """WebSocket real-time events example"""
    
    client = SlskrClient(
        base_url="http://localhost:8080",
        token="your-api-key-here",
        debug=True
    )
    
    try:
        # Connect to WebSocket
        ws = await client.connect_ws()
        
        # Register event listeners
        ws.on("message", handle_message)
        ws.on("search_update", handle_search_update)
        ws.on("transfer_update", handle_transfer_update)
        
        # Register connection listeners
        ws.on_connection_change(handle_connection_change)
        ws.on_error(handle_error)
        
        # Subscribe to topics
        ws.subscribe("messages", "search_updates", "transfer_updates")
        
        print(f"Subscribed to topics: {ws.get_subscribed_topics()}")
        print("Listening for events (press Ctrl+C to stop)...\n")
        
        # Keep running and receive events
        try:
            while ws.is_connected():
                await asyncio.sleep(1)
        except KeyboardInterrupt:
            print("\nStopping...")
        
        # Unsubscribe from topics
        ws.unsubscribe("messages")
        
        # Wait a bit for async cleanup
        await asyncio.sleep(0.5)
        
    except Exception as e:
        print(f"Error: {e}")
    finally:
        await client.close()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nInterrupted")
