import json
import socket
import sys
import time
from collections import Counter

import databento as db
from databento import Action, MBOMsg, Metadata

# from databento import DBNStore
from databento_dbn import DBNDecoder

from order_book import Market

HOST = "127.0.0.1"
PORT = 9001
RECV_CHUNK_SIZE = 81920


def handle_message(rec, count: int) -> None:
    # rec is a typed record object (for example an MBO message)
    # You can inspect fields: rec.header.ts_event, rec.order_id, rec.price, etc.
    print(f"[Msg #{count}] {rec}, {type(rec)}")


def run_consumer(host: str = HOST, port: int = PORT, handle_rec=handle_message) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        print(f"[consumer] Connecting to {host}:{port} ...", file=sys.stderr)
        sock.connect((host, port))
        print("[consumer] Connected.", file=sys.stderr)

        if 0:
            sock_file = sock.makefile("rb")
            store = db.DBNStore.from_bytes(sock_file)

            msg_count = 0

            def handle(rec):
                nonlocal handle_rec
                nonlocal msg_count
                msg_count += 1
                handle_rec(rec, msg_count)

            store.replay(handle)
        

        decoder = DBNDecoder()  # streaming decoder
        msg_count = 0

        try:
            while True:
                chunk = sock.recv(RECV_CHUNK_SIZE)
                if not chunk:
                    print("[consumer] Remote closed connection.")
                    break

                decoder.write(bytes(chunk))
                msgs = decoder.decode()

                for rec in msgs:
                    msg_count += 1
                    handle_rec(rec, msg_count)

        except KeyboardInterrupt:
            print("\n[consumer] Interrupted by user.", file=sys.stderr)

        finally:
            print("[consumer] Socket closed.", file=sys.stderr)
            print(f"[consumer] Total messages: {msg_count}", file=sys.stderr)


def handle_msg(rec, count: int) -> None:
    # rec is a typed record object (for example an MBO message)
    # You can inspect fields: rec.header.ts_event, rec.order_id, rec.price, etc.
    # print(f"[Msg #{count}] {rec}, {type(rec)}")
    if type(rec) == MBOMsg:
        print(
            f"[Msg #{count}] {rec.ts_event}, order_id={rec.order_id}, price={rec.price}, side={rec.side}, action={rec.action}"
        )


market = Market()
count_applied = 0
stats: Counter[int] = Counter()


def handle_rec(rec: MBOMsg, count: int) -> None:
    global count_applied
    if isinstance(rec, Metadata):
        return

    try:
        start_ns = time.perf_counter_ns()
        market.apply(rec)
        elapsed_us = (time.perf_counter_ns() - start_ns + 1_000 - 1) // 1_000
        stats[elapsed_us] += 1

        count_applied += 1

        if rec.action == Action.CANCEL:
            print(f"[Msg #{count}]  order_id={rec.order_id} -- canceled")
    except KeyError as e:
        print(f"Error applying message #{count}: {e=}, {rec=}", file=sys.stderr)


def percentiles_from_stats(stats: Counter[int], total: int, levels=(50, 90, 99)):
    result = {}
    sorted_us = sorted(stats.keys())
    cumulative = 0
    idx_targets = {p: total * p / 100.0 for p in levels}
    targets_done = set()

    for us in sorted_us:
        cumulative += stats[us]
        for p, idx in idx_targets.items():
            if p not in targets_done and cumulative >= idx:
                result[p] = us
                targets_done.add(p)
        if len(targets_done) == len(levels):
            break
    return result


if __name__ == "__main__":
    run_consumer(handle_rec=handle_rec)

    print([f"{stats[k]}:{k}us" for k in sorted(stats.keys())])
    print(f"Total applied messages: {count_applied}")

    pct = percentiles_from_stats(stats, count_applied)
    print(
        f"Latency percentiles (apply): "
        + ", ".join(f"p{p}={pct[p]}us" for p in sorted(pct))
    )

    snapshot = market.to_dict(include_orders=True)

    with open("order_book.json", "w") as f:
        json.dump(snapshot, f, indent=2)
