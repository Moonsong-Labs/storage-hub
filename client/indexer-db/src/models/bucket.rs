use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::bucket, DbConnection};

/// Table that holds the Buckets.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bucket)]
#[diesel(belongs_to(Msp, foreign_key = msp_id))]
pub struct Bucket {
    pub id: i32,
    pub msp_id: i32,
    pub account: String,
    pub blockchain_id: String,
    pub name: Vec<u8>,
    pub collection_id: Option<String>,
    pub private: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Bucket {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        msp_id: i32,
        account: String,
        blockchain_id: String,
        name: Vec<u8>,
        collection_id: Option<String>,
        private: bool,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::insert_into(bucket::table)
            .values((
                bucket::msp_id.eq(msp_id),
                bucket::account.eq(account),
                bucket::blockchain_id.eq(blockchain_id),
                bucket::name.eq(name),
                bucket::collection_id.eq(collection_id),
                bucket::private.eq(private),
            ))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }

    pub async fn update_privacy<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        bucket_id: String,
        collection_id: Option<String>,
        private: bool,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::update(bucket::table)
            .filter(bucket::account.eq(account))
            .filter(bucket::blockchain_id.eq(bucket_id))
            .set((
                bucket::collection_id.eq(collection_id),
                bucket::private.eq(private),
            ))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }

    pub async fn update_msp<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: String,
        msp_id: i32,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::update(bucket::table)
            .filter(bucket::blockchain_id.eq(bucket_id))
            .set(bucket::msp_id.eq(msp_id))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }
}
