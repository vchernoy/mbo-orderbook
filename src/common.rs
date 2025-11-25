use databento::dbn::{pretty, Action, MboMsg, Side};

/// Pretty-print a single MBO record.
pub fn print_pretty(idx: usize, mbo: &MboMsg) {
    // Decode action / side from raw bytes (i8 → u8 → enum)
    let action = Action::try_from(mbo.action as u8).unwrap_or(Action::None);
    let side = Side::try_from(mbo.side as u8).unwrap_or(Side::None);

    // Databento doc: prices are 1e-9 fixed precision units
    // let price = mbo.price as f64 / 1_000_000_000.0;

    println!(
        "#{:<6} ts_event={} instr_id={} oid={} px={:.2} qty={:<4} side={:?} action={:?}",
        idx,
        mbo.hd.ts_event,
        mbo.hd.instrument_id,
        mbo.order_id,
        pretty::Px(mbo.price),
        mbo.size,
        side,
        action,
    );
    // println!(
    //     "#{:<6} ts_event={} instr_id={} oid={} px={:.2} qty={:<4} side={:?} action={:?}",
    //     idx, pretty::Ts(mbo.hd.ts_event), mbo.hd.instrument_id, mbo.order_id, pretty::Px(mbo.price), mbo.size, side, action,
    // );
}
