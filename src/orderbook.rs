use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt::Display,
};

use databento::dbn::{pretty, Action, BidAskPair, MboMsg, Publisher, Record, Side, UNDEF_PRICE};

#[derive(Debug, Default)]
pub struct Market {
    books: HashMap<u32, Vec<(Publisher, Book)>>,
}

#[derive(Debug, Default)]
pub struct Book {
    orders_by_id: HashMap<u64, (Side, i64)>,
    offers: BTreeMap<i64, Level>,
    bids: BTreeMap<i64, Level>,
}

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: i64,
    pub size: u32,
    pub count: u32,
}

type Level = VecDeque<MboMsg>;

impl Market {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn books_by_pub(&self, instrument_id: u32) -> Option<&[(Publisher, Book)]> {
        self.books
            .get(&instrument_id)
            .map(|pub_books| pub_books.as_slice())
    }

    pub fn book(&self, instrument_id: u32, publisher: Publisher) -> Option<&Book> {
        let books = self.books.get(&instrument_id)?;
        books.iter().find_map(|(book_pub, book)| {
            if *book_pub == publisher {
                Some(book)
            } else {
                None
            }
        })
    }

    pub fn bbo(
        &self,
        instrument_id: u32,
        publisher: Publisher,
    ) -> (Option<PriceLevel>, Option<PriceLevel>) {
        self.book(instrument_id, publisher)
            .map(|book| book.bbo())
            .unwrap_or_default()
    }

    pub fn aggregated_bbo(&self, instrument_id: u32) -> (Option<PriceLevel>, Option<PriceLevel>) {
        let mut agg_bid = None;
        let mut agg_ask = None;
        let Some(books_by_pub) = self.books_by_pub(instrument_id) else {
            return (None, None);
        };
        for (_, book) in books_by_pub.iter() {
            let (bid, ask) = book.bbo();
            if let Some(bid) = bid {
                match &mut agg_bid {
                    None => agg_bid = Some(bid),
                    Some(ab) if bid.price > ab.price => agg_bid = Some(bid),
                    Some(ab) if bid.price == ab.price => {
                        ab.size += bid.size;
                        ab.count += bid.count;
                    }
                    Some(_) => {}
                }
            }
            if let Some(ask) = ask {
                match &mut agg_ask {
                    None => agg_ask = Some(ask),
                    Some(aa) if ask.price < aa.price => agg_ask = Some(ask),
                    Some(aa) if ask.price == aa.price => {
                        aa.size += ask.size;
                        aa.count += ask.count;
                    }
                    Some(_) => {}
                }
            }
        }
        (agg_bid, agg_ask)
    }

    pub fn apply(&mut self, mbo: MboMsg) {
        let publisher = mbo.publisher().unwrap();
        let books = self.books.entry(mbo.hd.instrument_id).or_default();
        let book = if let Some((_, book)) = books
            .iter_mut()
            .find(|(book_pub, _)| *book_pub == publisher)
        {
            book
        } else {
            books.push((publisher, Book::default()));
            &mut books.last_mut().unwrap().1
        };
        book.apply(mbo);
    }
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bbo(&self) -> (Option<PriceLevel>, Option<PriceLevel>) {
        (self.bid_level(0), self.ask_level(0))
    }

    pub fn bid_level(&self, idx: usize) -> Option<PriceLevel> {
        self.bids
            .iter()
            // Reverse to get highest first
            .rev()
            .nth(idx)
            .map(|(price, orders)| PriceLevel::new(*price, orders.iter()))
    }

    pub fn ask_level(&self, idx: usize) -> Option<PriceLevel> {
        self.offers
            .iter()
            .nth(idx)
            .map(|(price, orders)| PriceLevel::new(*price, orders.iter()))
    }

    pub fn bid_level_by_px(&self, px: i64) -> Option<PriceLevel> {
        self.bids
            .get(&px)
            .map(|orders| PriceLevel::new(px, orders.iter()))
    }

    pub fn ask_level_by_px(&self, px: i64) -> Option<PriceLevel> {
        self.offers
            .get(&px)
            .map(|orders| PriceLevel::new(px, orders.iter()))
    }

    pub fn order(&self, order_id: u64) -> Option<&MboMsg> {
        let (side, price) = self.orders_by_id.get(&order_id)?;
        let levels = self.side_levels(*side);
        let level = levels.get(price)?;
        level.iter().find(|order| order.order_id == order_id)
    }

    pub fn queue_pos(&self, order_id: u64) -> Option<u32> {
        let (side, price) = self.orders_by_id.get(&order_id)?;
        let levels = self.side_levels(*side);
        let level = levels.get(price)?;
        Some(
            level
                .iter()
                .take_while(|order| order.order_id != order_id)
                .fold(0, |acc, order| acc + order.size),
        )
    }

    pub fn snapshot(&self, level_count: usize) -> Vec<BidAskPair> {
        (0..level_count)
            .map(|i| {
                let mut ba_pair = BidAskPair::default();
                if let Some(bid) = self.bid_level(i) {
                    ba_pair.bid_px = bid.price;
                    ba_pair.bid_sz = bid.size;
                    ba_pair.bid_ct = bid.count;
                }
                if let Some(ask) = self.ask_level(i) {
                    ba_pair.ask_px = ask.price;
                    ba_pair.ask_sz = ask.size;
                    ba_pair.ask_ct = ask.count;
                }
                ba_pair
            })
            .collect()
    }

    pub fn apply(&mut self, mbo: MboMsg) {
        let action = mbo.action().unwrap();
        match action {
            Action::Modify => self.modify(mbo),
            Action::Trade | Action::Fill | Action::None => {}
            Action::Cancel => self.cancel(mbo),
            Action::Add => self.add(mbo),
            Action::Clear => self.clear(),
        }
    }

    fn clear(&mut self) {
        self.orders_by_id.clear();
        self.offers.clear();
        self.bids.clear();
    }

    fn add(&mut self, mbo: MboMsg) {
        let price = mbo.price;
        let side = mbo.side().unwrap();
        if mbo.flags.is_tob() {
            let levels: &mut BTreeMap<i64, Level> = self.side_levels_mut(side);
            levels.clear();
            // UNDEF_PRICE indicates the side's book should be cleared
            // and doesn't represent an order that should be added
            if mbo.price != UNDEF_PRICE {
                levels.insert(price, VecDeque::from([mbo]));
            }
        } else {
            assert_ne!(price, UNDEF_PRICE);
            assert!(self
                .orders_by_id
                .insert(mbo.order_id, (side, price))
                .is_none());
            let level: &mut Level = self.get_or_insert_level(side, price);
            level.push_back(mbo);
        }
    }

    fn cancel(&mut self, mbo: MboMsg) {
        let side = mbo.side().unwrap();
        let level = self.level_mut(side, mbo.price);
        let order_idx = Self::find_order(level, mbo.order_id);
        let existing_order = level.get_mut(order_idx).unwrap();
        assert!(existing_order.size >= mbo.size);
        existing_order.size -= mbo.size;
        if existing_order.size == 0 {
            level.remove(order_idx).unwrap();
            if level.is_empty() {
                self.remove_level(side, mbo.price);
            }
            self.orders_by_id.remove(&mbo.order_id).unwrap();
        }
    }

    fn modify(&mut self, mbo: MboMsg) {
        let order_id = mbo.order_id;
        let side = mbo.side().unwrap();
        let Some((id_side, id_price)) = self.orders_by_id.get_mut(&order_id) else {
            // If order not found, treat it as an add
            return self.add(mbo);
        };
        let prev_side = *id_side;
        let prev_price = *id_price;
        // Update orders by ID
        *id_side = side;
        *id_price = mbo.price;
        // Update level order
        let level = self.level_mut(prev_side, prev_price);
        let order_idx = Self::find_order(level, order_id);
        let existing_order = level.get_mut(order_idx).unwrap();
        existing_order.size = mbo.size;
        let should_keep_priority = prev_price == mbo.price && existing_order.size >= mbo.size;
        if should_keep_priority {
            return;
        }
        if prev_price != mbo.price {
            let prev_level = level;
            Self::remove_order(prev_level, order_id);
            if prev_level.is_empty() {
                self.remove_level(side, prev_price);
            }
            let level = self.get_or_insert_level(side, mbo.price);
            level.push_back(mbo);
        } else {
            Self::remove_order(level, order_id);
            level.push_back(mbo);
        }
    }

    fn get_or_insert_level(&mut self, side: Side, price: i64) -> &mut Level {
        let levels = self.side_levels_mut(side);
        levels.entry(price).or_default()
    }

    fn level_mut(&mut self, side: Side, price: i64) -> &mut Level {
        let levels = self.side_levels_mut(side);
        levels.get_mut(&price).unwrap()
    }

    fn remove_level(&mut self, side: Side, price: i64) {
        self.side_levels_mut(side).remove(&price).unwrap();
    }

    fn find_order(level: &VecDeque<MboMsg>, order_id: u64) -> usize {
        level.iter().position(|o| o.order_id == order_id).unwrap()
    }

    fn remove_order(level: &mut VecDeque<MboMsg>, order_id: u64) {
        let index = Self::find_order(level, order_id);
        level.remove(index).unwrap();
    }

    fn side_levels_mut(&mut self, side: Side) -> &mut BTreeMap<i64, Level> {
        match side {
            Side::Ask => &mut self.offers,
            Side::Bid => &mut self.bids,
            Side::None => panic!("Invalid side None"),
        }
    }

    fn side_levels(&self, side: Side) -> &BTreeMap<i64, Level> {
        match side {
            Side::Ask => &self.offers,
            Side::Bid => &self.bids,
            Side::None => panic!("Invalid side None"),
        }
    }
}

impl PriceLevel {
    fn new<'a>(price: i64, orders: impl Iterator<Item = &'a MboMsg>) -> Self {
        orders.fold(
            PriceLevel {
                price,
                size: 0,
                count: 0,
            },
            |mut level, order| {
                if !order.flags.is_tob() {
                    level.count += 1;
                }
                level.size += order.size;
                level
            },
        )
    }
}

impl Display for PriceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:4} @ {:6.2} | {:2} order(s)",
            self.size,
            pretty::Px(self.price),
            self.count
        )
    }
}
