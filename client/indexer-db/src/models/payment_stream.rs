use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::paymentstream, DbConnection};

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = paymentstream)]
pub struct PaymentStream {
    // postgres attributed ID for this payment stream
    pub id: i32,
    // Account ID of the payer
    pub account: String,
    // ID of the payee (msp or bsp)
    pub provider: String,
    // Total amount already paid to this provider from this account for this payment stream
    pub total_amount_paid: BigDecimal,
    // The last tick for which the payment stream has recorded a payment
    pub last_tick_charged: i64,
}

impl PaymentStream {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
    ) -> Result<Self, diesel::result::Error> {
        let ps = diesel::insert_into(paymentstream::table)
            .values((
                paymentstream::account.eq(account),
                paymentstream::provider.eq(provider),
            ))
            .returning(PaymentStream::as_select())
            .get_result(conn)
            .await?;
        Ok(ps)
    }

    pub async fn get<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        provider: String,
    ) -> Result<Self, diesel::result::Error> {
        // Looking by a payment stream by provider and the account associated
        let ps = paymentstream::table
            .filter(
                paymentstream::account
                    .eq(account)
                    .and(paymentstream::provider.eq(provider)),
            )
            .first(conn)
            .await?;
        Ok(ps)
    }

    pub async fn update_total_amount<'a>(
        conn: &mut DbConnection<'a>,
        ps_id: i32,
        new_total_amount: BigDecimal,
        last_tick_charged: i64,
        charged_at_tick: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(paymentstream::table)
            .filter(paymentstream::id.eq(ps_id))
            .set((
                paymentstream::total_amount_paid.eq(new_total_amount),
                paymentstream::last_tick_charged.eq(last_tick_charged),
                paymentstream::charged_at_tick.eq(charged_at_tick),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }
}
