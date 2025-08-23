#!/usr/bin/env python3
"""
Realtime vs Batch Processing Example

This example demonstrates the difference between traditional batch processing
and realtime processing using RealtimeDecoder. It shows how RealtimeDecoder
maintains state across individual messages, enabling real-time processing
without losing the context that makes batch processing effective.
"""

import time
from dataclasses import dataclass
import pandas as pd
from pathlib import Path

# Import rs1090
try:
    import rs1090
    from rs1090 import RealtimeDecoder, Message
except ImportError:
    print("Error: rs1090 Python bindings not available.")
    print("Please build the Python bindings first:")
    print("  cd python && uv run maturin develop")
    exit(1)

@dataclass
class RawMessage:
    timestamp: float
    rawmsg: str


def load_data() -> list[RawMessage]:
    """Load ADS-B data from CSV file. Returns list of Message objects"""
    data_file = Path("../../crates/rs1090/data/short_flights.csv")
    
    if not data_file.exists():
        print(f"Error: Data file not found: {data_file}")
        print("Please run this script from the python/examples directory")
        exit(1)
    
    print(f"Loading data from: {data_file}")
    df = pd.read_csv(data_file, names=['timestamp', 'rawmsg'])
    print(f"Loaded {len(df)} ADS-B messages")
    
    return [RawMessage(row.timestamp, row.rawmsg) for row in df.itertuples()]


def batch_processing(messages: list[RawMessage]) -> tuple[list[Message], float]:
    """
    Process all messages at once using rs1090.decode. 
    Returns decoded messages and processing time.
    """    
    # Process all messages at once
    message_list = [message.rawmsg for message in messages]
    timestamps = [message.timestamp for message in messages]
    
    start_time = time.time()
    decoded = rs1090.decode(message_list, timestamps)
    
    processing_time = time.time() - start_time
    
    return decoded, processing_time


def realtime_processing(messages: list[RawMessage]) -> tuple[list[Message], float, int]:
    """
    Process messages one by one using RealtimeDecoder.
    Returns (decoded messages, processing time, aircraft count).
    """    
    # Create RealtimeDecoder
    decoder = RealtimeDecoder()
    
    start_time = time.time()
    
    # Process messages one by one (simulating real-time arrival)
    decoded = []
    
    for message in messages:
        # Decode single message (timestamp is optional, defaults to current time)
        decoded_message = decoder.decode(message.rawmsg, message.timestamp)
        
        if decoded_message:
            decoded.append(decoded_message)
    
    processing_time = time.time() - start_time
    aircraft_count = decoder.aircraft_count()
    
    return decoded, processing_time, aircraft_count

def compare(batch_decoded: list[Message], realtime_decoded: list[Message]) -> tuple[int, int, bool]:
    """
    Compare batch and realtime decoding results.
    Returns (total_messages, position_messages, messages_match).
    """
    if len(batch_decoded) != len(realtime_decoded):
        print(f"❌ Different number of messages decoded: {len(batch_decoded)} vs {len(realtime_decoded)}")
        return 0, 0, False
    
    # Count position messages (messages with latitude)
    batch_positions = sum(1 for msg in batch_decoded if msg and msg.get('latitude'))
    realtime_positions = sum(1 for msg in realtime_decoded if msg and msg.get('latitude'))
    
    # Check that we got the same number of positions
    if batch_positions != realtime_positions:
        print(f"❌ Different number of positions: {batch_positions} vs {realtime_positions}")
        return len(batch_decoded), batch_positions, False
    
    # Compare each message for content equality (ignoring wrapper fields)
    for i, (batch_msg, realtime_msg) in enumerate(zip(batch_decoded, realtime_decoded)):
        # Extract core message fields for comparison (ignore timestamp, frame, metadata)
        batch_core = {k: v for k, v in batch_msg.items() if k not in ['timestamp', 'frame', 'metadata']}
        realtime_core = realtime_msg
        
        if batch_core != realtime_core:
            print(f"❌ Message {i} differs:")
            print(f"   Batch: {batch_core}")
            print(f"   Realtime: {realtime_core}")
            return len(batch_decoded), batch_positions, False
    
    return len(batch_decoded), batch_positions, True

def main():
    """Compare batch vs realtime processing"""
    print("Realtime vs Batch Processing Comparison")
    print("=" * 50)
    
    # Load data
    messages = load_data()
    
    # Process messages using both methods
    batch_decoded, batch_time = batch_processing(messages)
    realtime_decoded, realtime_time, aircraft_count = realtime_processing(messages)
    
    # Compare results
    total_messages, position_messages, messages_match = compare(batch_decoded, realtime_decoded)
    
    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    print(f"Messages processed: {len(messages)}")
    print(f"Messages decoded: {total_messages}")
    print(f"Position messages: {position_messages}")
    print(f"Aircraft tracked: {aircraft_count}")
    print()
    print(f"Batch processing:    {batch_time:.3f}s ({len(messages) / batch_time:.0f} msg/s)")
    print(f"Realtime processing: {realtime_time:.3f}s ({len(messages) / realtime_time:.0f} msg/s)")
    
    # Calculate percentage difference
    if batch_time > 0:
        percent_diff = ((realtime_time - batch_time) / batch_time) * 100
        if percent_diff > 0:
            print(f"Performance: {percent_diff:.1f}% slower")
        elif percent_diff < 0:
            print(f"Performance: {abs(percent_diff):.1f}% faster")
        else:
            print(f"Performance: identical")
    
    if messages_match:
        print(f"\n✅ All messages match exactly!")
    else:
        print(f"\n❌ Differences detected in decoded messages")

if __name__ == "__main__":
    main() 