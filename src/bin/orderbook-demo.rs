use mbo_orderbook::orderbook::Market;

use databento::{
    dbn::{
        decode::{AsyncDbnDecoder, DbnMetadata},
        Dataset, MboMsg, Schema, SymbolIndex,
    },
    historical::timeseries::GetRangeToFileParams,
    HistoricalClient,
};
use time::macros::datetime;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), databento::Error> {
    let mut client = HistoricalClient::builder().key_from_env()?.build()?;
    let mut market = Market::default();
    let path = "dbeq-basic-20240403.mbo.dbn.zst";
    if !fs::try_exists(path).await? {
        client
            .timeseries()
            .get_range_to_file(
                &GetRangeToFileParams::builder()
                    .dataset(Dataset::DbeqBasic)
                    .symbols(vec!["GOOG", "GOOGL"])
                    .date_time_range(
                        datetime!(2024-04-03 08:00:00 UTC)..datetime!(2024-04-03 14:00:00 UTC),
                    )
                    .schema(Schema::Mbo)
                    .path(path)
                    .build(),
            )
            .await?;
    };
    let mut decoder = AsyncDbnDecoder::from_zstd_file(path).await?;
    let symbol_map = decoder.metadata().symbol_map()?;
    while let Some(mbo) = decoder.decode_record::<MboMsg>().await? {
        market.apply(mbo.clone());
        // If it's the last update in an event, print the state of the aggregated book
        if mbo.flags.is_last() {
            let symbol = symbol_map.get_for_rec(mbo).unwrap();
            let (best_bid, best_offer) = market.aggregated_bbo(mbo.hd.instrument_id);
            println!("{symbol} Aggregated BBO | {}", mbo.ts_recv().unwrap());
            if let Some(best_offer) = best_offer {
                println!("    {best_offer}");
            } else {
                println!("    None");
            }
            if let Some(best_bid) = best_bid {
                println!("    {best_bid}");
            } else {
                println!("    None");
            }
        }
    }
    Ok(())
}
