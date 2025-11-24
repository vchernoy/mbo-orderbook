from databento import DBNStore

path = "CLX5_mbo.dbn"
store = DBNStore.from_file(path)


def handle_message(rec, count: int) -> None:
    # rec is a typed record object (for example an MBO message)
    # You can inspect fields: rec.header.ts_event, rec.order_id, rec.price, etc.
    print(f"[Msg #{count}] {rec}, {type(rec)}")


idx = 0


def handle(rec):
    global idx
    handle_message(rec, idx)
    idx += 1


store.replay(handle)

if 0:
    for idx, msg in enumerate(store):
        print(idx, msg)

    print(type(msg))

    print("Dataset:", store.dataset)
    print("Schema:", store.schema)
    print("Metadata:", store.metadata)
