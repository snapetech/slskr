#!/usr/bin/env python3
"""
Integration example for soulseekR - demonstrates coordinated client operations
Shows batch operations, WebSocket events, error handling, and data aggregation
"""

import asyncio
import time
from soulseekr import SoulseekrClient, ApiError


class SearchCoordinator:
    """Coordinates multiple search operations"""

    def __init__(self, client):
        self.client = client
        self.searches = {}
        self.results_count = {}

    async def create_searches(self, queries):
        """Create multiple searches concurrently"""
        print(f"\n=== Creating {len(queries)} searches ===")
        
        tasks = [self.client.create_search(q) for q in queries]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        successful = []
        for query, result in zip(queries, results):
            if isinstance(result, Exception):
                print(f"✗ Error searching '{query}': {result}")
            else:
                search_id = result.get('id')
                self.searches[search_id] = query
                successful.append(search_id)
                print(f"✓ Created search: {query} (ID: {search_id})")

        return successful

    async def monitor_searches(self, search_ids, duration_seconds=5):
        """Monitor search progress"""
        print(f"\n=== Monitoring {len(search_ids)} searches for {duration_seconds}s ===")
        
        start = time.time()
        while time.time() - start < duration_seconds:
            tasks = [
                self.client.get_search_details(sid, limit=10)
                for sid in search_ids
            ]
            
            try:
                details_list = await asyncio.gather(*tasks, return_exceptions=True)
                
                for search_id, details in zip(search_ids, details_list):
                    if isinstance(details, Exception):
                        continue
                    
                    result_count = len(details.get('results', []))
                    self.results_count[search_id] = result_count
                    query = self.searches.get(search_id, 'Unknown')
                    print(f"  {query}: {result_count} results")
                
                await asyncio.sleep(1)
            except Exception as e:
                print(f"Error monitoring searches: {e}")
                break

    async def get_top_results(self, search_id, limit=5):
        """Get top results from a search"""
        try:
            details = await self.client.get_search_details(search_id, limit=limit)
            return details.get('results', [])
        except ApiError as e:
            print(f"Error getting search details: {e}")
            return []


class MessageHandler:
    """Handles message operations"""

    def __init__(self, client):
        self.client = client
        self.messages_sent = 0
        self.messages_received = 0

    async def list_recent_messages(self, limit=10):
        """List recent messages"""
        try:
            messages = await self.client.list_messages(limit=limit)
            self.messages_received = len(messages)
            return messages
        except ApiError as e:
            print(f"Error listing messages: {e}")
            return []

    async def send_bulk_messages(self, recipients_messages):
        """Send multiple messages"""
        print(f"\n=== Sending {len(recipients_messages)} messages ===")
        
        tasks = [
            self.client.send_message(recipient, content)
            for recipient, content in recipients_messages
        ]
        
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        successful = 0
        for (recipient, _), result in zip(recipients_messages, results):
            if isinstance(result, Exception):
                print(f"✗ Failed to send message to {recipient}: {result}")
            else:
                successful += 1
                print(f"✓ Message sent to {recipient}")
        
        self.messages_sent = successful
        return successful


async def batch_analytics(client):
    """Gather analytics using batch operations"""
    print("\n=== Batch Analytics ===")
    
    batch = client.batch.builder()
    batch.get("/api/stats", op_id="stats")
    batch.get("/api/config", op_id="config")
    batch.get("/api/capabilities", op_id="caps")

    response = await batch.execute()

    analytics = {}
    for result in response.results:
        if result.is_success():
            analytics[result.id] = result.body
            print(f"✓ {result.id}: collected")
        else:
            print(f"✗ {result.id}: failed with status {result.status}")

    return analytics


async def websocket_monitoring(client, duration_seconds=3):
    """Monitor events via WebSocket"""
    print(f"\n=== WebSocket Monitoring for {duration_seconds}s ===")
    
    try:
        ws = await client.connect_ws()
        
        # Subscribe to all event types
        ws.subscribe("messages", "search_updates", "transfer_updates")
        
        event_counts = {"messages": 0, "search_updates": 0, "transfer_updates": 0}
        
        # Simulate event reception
        async def event_receiver():
            start = time.time()
            while time.time() - start < duration_seconds:
                await asyncio.sleep(0.5)
            return event_counts

        result = await event_receiver()
        print(f"Event monitoring complete: {result}")
        
    except Exception as e:
        print(f"Error with WebSocket: {e}")
    finally:
        await client.disconnect_ws()


async def transfer_operations(client):
    """Handle transfer operations"""
    print("\n=== Transfer Operations ===")
    
    try:
        # List active transfers
        transfers = await client.list_transfers(
            direction="download",
            status="active",
            limit=10
        )
        
        print(f"Active downloads: {len(transfers)}")
        
        for transfer in transfers[:3]:
            print(f"  - {transfer.get('filename')}: {transfer.get('progress')}%")
        
        return len(transfers)
    except ApiError as e:
        print(f"Error listing transfers: {e}")
        return 0


async def main():
    """Main integration example"""
    
    # Initialize client
    client = SoulseekrClient(
        base_url="http://localhost:8080",
        token="your-api-key-here",
        debug=True
    )

    try:
        # 1. Verify server connectivity
        print("=== Server Verification ===")
        health = await client.health()
        print(f"✓ Server status: {health.get('status')}")

        # 2. Search coordination
        coordinator = SearchCoordinator(client)
        queries = ["radiohead", "pink floyd", "led zeppelin"]
        search_ids = await coordinator.create_searches(queries)

        if search_ids:
            # Monitor the searches
            await coordinator.monitor_searches(search_ids, duration_seconds=3)

            # Get top results from first search
            if search_ids:
                top_results = await coordinator.get_top_results(search_ids[0], limit=5)
                print(f"\nTop results for first search: {len(top_results)}")

        # 3. Message operations
        handler = MessageHandler(client)
        recent_messages = await handler.list_recent_messages(limit=5)
        print(f"\n=== Message Operations ===")
        print(f"Recent messages: {len(recent_messages)}")

        # Send demo messages
        demo_recipients = [
            ("demo_user_1", "Test message 1"),
            ("demo_user_2", "Test message 2"),
        ]
        sent_count = await handler.send_bulk_messages(demo_recipients)
        print(f"Messages sent: {sent_count}")

        # 4. Batch analytics
        analytics = await batch_analytics(client)
        print(f"Collected {len(analytics)} analytics")

        # 5. Transfer monitoring
        active_transfers = await transfer_operations(client)

        # 6. WebSocket monitoring
        await websocket_monitoring(client, duration_seconds=2)

        # 7. Summary
        print("\n=== Integration Summary ===")
        print(f"Searches created: {len(search_ids)}")
        print(f"Messages sent: {handler.messages_sent}")
        print(f"Analytics collected: {len(analytics)}")
        print(f"Active transfers: {active_transfers}")
        print("✓ Integration test complete")

    except Exception as e:
        print(f"Integration error: {e}")
        import traceback
        traceback.print_exc()

    finally:
        await client.close()


if __name__ == "__main__":
    asyncio.run(main())
