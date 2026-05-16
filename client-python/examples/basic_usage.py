#!/usr/bin/env python3
"""
Basic usage example for slskr Python client
"""

import asyncio
from slskr import SlskrClient


async def main():
    """Basic client usage example"""
    
    # Initialize client
    client = SlskrClient(
        base_url="http://127.0.0.1:5030",
        token="your-api-key-here",
        debug=True  # Enable debug logging
    )
    
    try:
        # Check server health
        health = await client.health()
        print(f"Server status: {health.get('status')}")
        
        # Get API version
        version = await client.version()
        print(f"API version: {version.get('version')}")
        
        # Get capabilities
        caps = await client.get_capabilities()
        print(f"Endpoints available: {len(caps.get('endpoints', []))}")
        
        # Get server stats
        stats = await client.get_stats()
        print(f"Total searches: {stats.get('total_searches')}")
        print(f"Active transfers: {stats.get('active_transfers')}")
        
        # List searches
        searches = await client.list_searches(limit=10)
        print(f"\nFound {len(searches)} recent searches")
        for search in searches[:3]:
            print(f"  - Query: {search.get('query')}")
            print(f"    Status: {search.get('status')}")
        
        # Create a new search
        new_search = await client.create_search(query="beethoven symphony")
        print(f"\nCreated search: {new_search.get('id')}")
        print(f"Query: {new_search.get('query')}")
        
        # Get search details
        search_id = new_search.get('id')
        details = await client.get_search_details(search_id, limit=20)
        print(f"Search results: {len(details.get('results', []))}")
        
        # List messages
        messages = await client.list_messages(limit=5)
        print(f"\nMessages: {len(messages)}")
        for msg in messages[:2]:
            print(f"  - From: {msg.get('username')}")
            print(f"    Content: {msg.get('content')[:50]}...")
        
        # List transfers
        transfers = await client.list_transfers(limit=5)
        print(f"\nTransfers: {len(transfers)}")
        for transfer in transfers[:2]:
            print(f"  - File: {transfer.get('filename')}")
            print(f"    Status: {transfer.get('status')}")
            
    except Exception as e:
        print(f"Error: {e}")
    finally:
        await client.close()


if __name__ == "__main__":
    asyncio.run(main())
