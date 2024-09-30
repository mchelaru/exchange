use anyhow::{bail, Result};
use market::Market;
use oep::{
    cancel::{Cancel, CANCEL_SIZE},
    decoder::Decoder,
    execution_report::ExecutionReport,
    modify::{Modify, MODIFY_SIZE},
    neworder::{NewOrder, NEWORDER_SIZE},
    oep_message::OepMessage,
    sessioninfo::{SessionInfo, SESSIONINFO_SIZE},
};
use order::{Order, OrderState};

pub enum MessageWrapper {
    NewOrder(NewOrder),
    Modify(Modify),
    Cancel(Cancel),
    KillSession(SessionInfo),
}

static HEADER_SIZE: usize = 4;

#[must_use]
pub fn decode_message(buffer: &[u8]) -> Result<(MessageWrapper, u64)> {
    match buffer[0] {
        0 => {
            assert_eq!(HEADER_SIZE + NEWORDER_SIZE, buffer.len());
            let o = NewOrder::decode(
                buffer[HEADER_SIZE..HEADER_SIZE + NEWORDER_SIZE]
                    .try_into()
                    .expect("new order buffer try_into failed"),
            )
            .expect("decoding new order");
            let instrument = o.book_id;
            Ok((MessageWrapper::NewOrder(o), instrument))
        }
        1 => {
            assert_eq!(HEADER_SIZE + MODIFY_SIZE, buffer.len());
            let o = Modify::decode(
                buffer[HEADER_SIZE..HEADER_SIZE + MODIFY_SIZE]
                    .try_into()
                    .expect("modify buffer try_into failed"),
            )
            .expect("decoding modify");
            let instrument = o.book_id;
            Ok((MessageWrapper::Modify(o), instrument))
        }
        2 => {
            assert_eq!(HEADER_SIZE + CANCEL_SIZE, buffer.len());
            let o = Cancel::decode(
                buffer[HEADER_SIZE..HEADER_SIZE + CANCEL_SIZE]
                    .try_into()
                    .expect("cancel buffer try_into failed"),
            )
            .expect("decoding cancel");
            let instrument = o.book_id;
            Ok((MessageWrapper::Cancel(o), instrument))
        }
        3 => {
            assert_eq!(HEADER_SIZE + SESSIONINFO_SIZE, buffer.len());
            let o = SessionInfo::decode(
                buffer[HEADER_SIZE..HEADER_SIZE + SESSIONINFO_SIZE]
                    .try_into()
                    .expect("session info buffer try_into failed"),
            )
            .expect("decoding kill session");
            Ok((MessageWrapper::KillSession(o), 0))
        }
        k => bail!("Invalid message type {k}"),
    }
}

#[must_use]
/// process a message in the supplied market and returns an execution report
///
/// # Example
///
/// ```
/// use std::{cell::RefCell, rc::Rc};
/// use disseminator::mockdisseminator::MockDisseminator;
/// use instruments::instrument::{Instrument, InstrumentState, InstrumentType};
/// use market::Market;
/// use oep::{
///     execution_report::ExecutionReport, neworder::NewOrder,
///     oep_message::OepMessage,
/// };
/// use order::{OrderState, Side};
/// use matching_engine::processor::MessageWrapper;
/// use matching_engine::processor::process_message;
///
/// const BOOK_ID: u64 = 1234;
/// let instrument = Instrument::new(
///         BOOK_ID,
///         "TEST",
///         InstrumentType::Share,
///         InstrumentState::Trading,
///         10,
///         20,
///     );
/// let mut market = Market::new(
///         Rc::new(RefCell::new(instrument)),
///         Rc::new(RefCell::new(MockDisseminator::new())),
///     );
/// let new_order = MessageWrapper::NewOrder(NewOrder {
///         client_order_id: 7000,
///         participant: 123,
///         book_id: BOOK_ID,
///         quantity: 200,
///         price: 100,
///         order_type: 0,
///         side: 1,
///         gateway_id: 15,
///         session_id: 2,
///     });
/// let execution_report = process_message(&mut market, new_order);
/// assert_eq!(execution_report.state, OrderState::Inserted.into());
/// ```
///
pub fn process_message(market: &mut Market, msg: MessageWrapper) -> ExecutionReport {
    match msg {
        MessageWrapper::NewOrder(m) => {
            if market.get_instrument().borrow().get_id() != m.book_id || m.get_participant() == 0 {
                return ExecutionReport {
                    participant: m.participant,
                    order_id: m.client_order_id,
                    submitted_order_id: m.client_order_id,
                    book: m.book_id,
                    quantity: m.quantity,
                    price: m.price,
                    flags: 0,
                    side: m.side,
                    state: OrderState::Rejected.into(),
                    gateway_id: m.gateway_id,
                    session_id: m.session_id,
                };
            }

            let o = Order::new(
                m.get_participant(),
                market.get_instrument().clone(),
                m.price,
                m.quantity,
                m.side.into(),
                m.order_type.into(),
                m.get_gateway_id(),
                m.get_session_id(),
            );
            let (state, id) = market.add_order(o);
            // publish back the execution report
            ExecutionReport {
                participant: m.participant,
                order_id: id,
                submitted_order_id: m.client_order_id,
                book: m.book_id,
                quantity: m.quantity,
                price: m.price,
                flags: 0,
                side: m.side,
                state: state.into(),
                gateway_id: m.gateway_id,
                session_id: m.session_id,
            }
        }
        MessageWrapper::Modify(m) => {
            if market.get_instrument().borrow().get_id() != m.book_id || m.get_participant() == 0 {
                return ExecutionReport {
                    participant: m.participant,
                    order_id: m.order_id,
                    submitted_order_id: m.order_id,
                    book: m.book_id,
                    quantity: m.quantity,
                    price: m.price,
                    flags: 0,
                    side: m.side,
                    state: OrderState::Rejected.into(),
                    gateway_id: m.gateway_id,
                    session_id: m.session_id,
                };
            }

            let mut o = Order::new(
                m.get_participant(),
                market.get_instrument().clone(),
                m.price,
                m.quantity,
                m.get_side().into(),
                order::OrderType::Day, // FIXME:
                m.get_gateway_id(),
                m.get_session_id(),
            );
            o.set_id(m.order_id);
            let (state, id) = market.modify_order(o);
            ExecutionReport {
                participant: m.participant,
                order_id: id,
                submitted_order_id: m.order_id,
                book: m.book_id,
                quantity: m.quantity,
                price: m.price,
                flags: 0, // FIXME:
                side: m.get_side().into(),
                state: state.into(),
                gateway_id: m.gateway_id,
                session_id: m.session_id,
            }
        }
        MessageWrapper::Cancel(m) => {
            if market.get_instrument().borrow().get_id() != m.book_id || m.get_participant() == 0 {
                return ExecutionReport {
                    participant: m.participant,
                    order_id: m.order_id,
                    submitted_order_id: m.order_id,
                    book: m.book_id,
                    quantity: 0,
                    price: 0,
                    flags: 0,
                    side: m.side,
                    state: OrderState::Rejected.into(),
                    gateway_id: m.gateway_id,
                    session_id: m.session_id,
                };
            }

            let mut o = Order::new(
                m.participant,
                market.get_instrument().clone(),
                0,
                0,
                m.get_side().into(),
                order::OrderType::Day, // FIXME:
                m.get_gateway_id(),
                m.get_session_id(),
            );
            o.set_id(m.order_id);
            let state = market.cancel_order(o);
            ExecutionReport {
                participant: m.participant,
                order_id: m.order_id,
                submitted_order_id: m.order_id,
                book: m.book_id,
                quantity: 0,
                price: 0,
                flags: 0,
                side: m.get_side().into(),
                state: state.into(),
                gateway_id: m.gateway_id,
                session_id: m.session_id,
            }
        }
        MessageWrapper::KillSession(_session_info) => todo!(),
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use disseminator::mockdisseminator::MockDisseminator;
    use instruments::instrument::{Instrument, InstrumentState, InstrumentType};
    use market::Market;
    use oep::{
        cancel::Cancel, execution_report::ExecutionReport, modify::Modify, neworder::NewOrder,
        oep_message::OepMessage,
    };
    use order::{OrderState, OrderType, Side};

    use super::{process_message, MessageWrapper};

    const BOOK_ID: u64 = 10000;

    fn default_market() -> Market {
        let instrument = Instrument::new(
            BOOK_ID,
            "TEST",
            InstrumentType::Share,
            InstrumentState::Trading,
            10,
            20,
        );
        Market::new(
            Rc::new(RefCell::new(instrument)),
            Rc::new(RefCell::new(MockDisseminator::new())),
        )
    }

    const DEFAULT_GATEWAY_ID: u8 = 15;
    const DEFAULT_SESSION_ID: u32 = 2500;

    fn process_default_day_order(market: &mut Market) -> ExecutionReport {
        let new_order = MessageWrapper::NewOrder(NewOrder {
            client_order_id: 7000,
            participant: 123,
            book_id: BOOK_ID,
            quantity: 200,
            price: 100,
            order_type: OrderType::Day.into(),
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        process_message(market, new_order)
    }

    #[test]
    fn process_new_order() {
        let mut market = default_market();
        let ereport = process_default_day_order(&mut market);

        assert_eq!(BOOK_ID, ereport.get_book());
        assert_eq!(ereport.state, OrderState::Inserted.into());
        assert_eq!(1, ereport.side);
        assert_eq!(100, ereport.get_price());
        assert_eq!(DEFAULT_GATEWAY_ID, ereport.get_gateway_id());
        assert_eq!(123, ereport.get_participant());
        assert_eq!(DEFAULT_SESSION_ID, ereport.get_session_id());
        assert_eq!(7000, ereport.get_submitted_order_id());
        assert_eq!(1, ereport.get_order_id());
    }

    #[test]
    fn process_modify() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(2, ereport.get_order_id());
        assert_eq!(15, ereport.get_quantity());
        assert_eq!(12, ereport.get_price());
        assert_eq!(ereport.state, OrderState::Inserted.into());
    }

    #[test]
    fn process_modify_wrong_participant() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123 + 5,
            order_id,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(0, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn participant_zero_rejected() {
        let mut market = default_market();
        let new_order = MessageWrapper::NewOrder(NewOrder {
            client_order_id: 7000,
            participant: 0,
            book_id: BOOK_ID,
            quantity: 200,
            price: 100,
            order_type: OrderType::Day.into(),
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        assert_eq!(
            process_message(&mut market, new_order).state,
            OrderState::Rejected.into()
        );
    }

    #[test]
    fn process_modify_wrong_order_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id: order_id + 5,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(0, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_modify_wrong_book_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id,
            book_id: BOOK_ID + 5,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(order_id, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_modify_wrong_gateway_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID + 5,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(0, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_modify_wrong_session_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID + 5,
            side: Side::Ask.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(0, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_modify_wrong_side() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let modify_order = MessageWrapper::Modify(Modify {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            quantity: 15,
            price: 12,
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
            side: Side::Bid.into(),
        });

        let ereport = process_message(&mut market, modify_order);

        assert_eq!(0, ereport.get_order_id());
        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Cancelled.into());
    }

    #[test]
    fn process_cancel_wrong_participant() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123 + 5,
            order_id,
            book_id: BOOK_ID,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel_wrong_order_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id: order_id + 5,
            book_id: BOOK_ID,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel_wrong_book_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id,
            book_id: BOOK_ID + 5,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel_wrong_side() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            side: Side::Bid.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel_wrong_gateway_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID + 5,
            session_id: DEFAULT_SESSION_ID,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }

    #[test]
    fn process_cancel_wrong_session_id() {
        let mut market = default_market();

        let order_id = process_default_day_order(&mut market).order_id;
        assert_eq!(1, order_id);

        let cancel_order = MessageWrapper::Cancel(Cancel {
            participant: 123,
            order_id,
            book_id: BOOK_ID,
            side: Side::Ask.into(),
            gateway_id: DEFAULT_GATEWAY_ID,
            session_id: DEFAULT_SESSION_ID + 5,
        });

        let ereport = process_message(&mut market, cancel_order);

        assert_eq!(ereport.state, OrderState::Rejected.into());
    }
}
