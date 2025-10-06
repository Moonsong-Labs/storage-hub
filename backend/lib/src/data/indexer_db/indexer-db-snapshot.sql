--
-- PostgreSQL database dump
--

-- Dumped from database version 15.14 (Debian 15.14-1.pgdg13+1)
-- Dumped by pg_dump version 15.13
-- Dumped after `move-bucket-test.ts` with `pg_dump "postgresql://postgres:postgres@localhost:5432/storage_hub" --format=plain --column-inserts --no-owner --no-privileges -b -f indexer-db-snapshot.sql`

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$;


--
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: bsp; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bsp (
    id bigint NOT NULL,
    account character varying NOT NULL,
    capacity numeric(20,0) NOT NULL,
    stake numeric(38,0) DEFAULT 0 NOT NULL,
    last_tick_proven bigint DEFAULT 0 NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    onchain_bsp_id character varying NOT NULL,
    merkle_root bytea NOT NULL
);


--
-- Name: bsp_file; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bsp_file (
    bsp_id bigint NOT NULL,
    file_id bigint NOT NULL
);


--
-- Name: bsp_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.bsp_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: bsp_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.bsp_id_seq OWNED BY public.bsp.id;


--
-- Name: bsp_multiaddress; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bsp_multiaddress (
    bsp_id bigint NOT NULL,
    multiaddress_id bigint NOT NULL
);


--
-- Name: bucket; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bucket (
    id bigint NOT NULL,
    msp_id bigint,
    account character varying NOT NULL,
    onchain_bucket_id bytea NOT NULL,
    name bytea NOT NULL,
    collection_id character varying,
    private boolean NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    merkle_root bytea NOT NULL
);


--
-- Name: bucket_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.bucket_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: bucket_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.bucket_id_seq OWNED BY public.bucket.id;


--
-- Name: file; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.file (
    id bigint NOT NULL,
    account bytea NOT NULL,
    file_key bytea NOT NULL,
    bucket_id bigint NOT NULL,
    onchain_bucket_id bytea NOT NULL,
    location bytea NOT NULL,
    fingerprint bytea NOT NULL,
    size bigint NOT NULL,
    step integer NOT NULL,
    deletion_status integer,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: file_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.file_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: file_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.file_id_seq OWNED BY public.file.id;


--
-- Name: file_peer_id; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.file_peer_id (
    file_id bigint NOT NULL,
    peer_id bigint NOT NULL
);


--
-- Name: msp; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.msp (
    id bigint NOT NULL,
    account character varying NOT NULL,
    capacity numeric(20,0) NOT NULL,
    value_prop character varying NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    onchain_msp_id character varying NOT NULL
);


--
-- Name: msp_file; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.msp_file (
    msp_id bigint NOT NULL,
    file_id bigint NOT NULL
);


--
-- Name: msp_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.msp_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: msp_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.msp_id_seq OWNED BY public.msp.id;


--
-- Name: msp_multiaddress; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.msp_multiaddress (
    msp_id bigint NOT NULL,
    multiaddress_id bigint NOT NULL
);


--
-- Name: multiaddress; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.multiaddress (
    id bigint NOT NULL,
    address bytea NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: multiaddress_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.multiaddress_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: multiaddress_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.multiaddress_id_seq OWNED BY public.multiaddress.id;


--
-- Name: paymentstream; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.paymentstream (
    id bigint NOT NULL,
    account character varying NOT NULL,
    provider character varying NOT NULL,
    total_amount_paid numeric(38,0) DEFAULT 0 NOT NULL,
    last_tick_charged bigint DEFAULT 0 NOT NULL,
    charged_at_tick bigint DEFAULT 0 NOT NULL,
    rate numeric(38,0),
    amount_provided numeric(38,0),
    CONSTRAINT check_payment_stream_type CHECK ((((rate IS NOT NULL) AND (amount_provided IS NULL)) OR ((rate IS NULL) AND (amount_provided IS NOT NULL))))
);


--
-- Name: paymentstream_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.paymentstream_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: paymentstream_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.paymentstream_id_seq OWNED BY public.paymentstream.id;


--
-- Name: peer_id; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.peer_id (
    id bigint NOT NULL,
    peer bytea NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: peer_id_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.peer_id_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: peer_id_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.peer_id_id_seq OWNED BY public.peer_id.id;


--
-- Name: service_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.service_state (
    id integer NOT NULL,
    last_indexed_finalized_block bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT service_state_id_check CHECK ((id = 1))
);


--
-- Name: bsp id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp ALTER COLUMN id SET DEFAULT nextval('public.bsp_id_seq'::regclass);


--
-- Name: bucket id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bucket ALTER COLUMN id SET DEFAULT nextval('public.bucket_id_seq'::regclass);


--
-- Name: file id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.file ALTER COLUMN id SET DEFAULT nextval('public.file_id_seq'::regclass);


--
-- Name: msp id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp ALTER COLUMN id SET DEFAULT nextval('public.msp_id_seq'::regclass);


--
-- Name: multiaddress id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.multiaddress ALTER COLUMN id SET DEFAULT nextval('public.multiaddress_id_seq'::regclass);


--
-- Name: paymentstream id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.paymentstream ALTER COLUMN id SET DEFAULT nextval('public.paymentstream_id_seq'::regclass);


--
-- Name: peer_id id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.peer_id ALTER COLUMN id SET DEFAULT nextval('public.peer_id_id_seq'::regclass);


--
-- Data for Name: __diesel_schema_migrations; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('00000000000000', '2025-10-03 18:50:08.458999');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240920035333', '2025-10-03 18:50:08.461168');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240926132546', '2025-10-03 18:50:08.466104');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240926145832', '2025-10-03 18:50:08.473292');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240927112918', '2025-10-03 18:50:08.484333');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240927125722', '2025-10-03 18:50:08.495687');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20240927152605', '2025-10-03 18:50:08.499038');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20241001112655', '2025-10-03 18:50:08.510338');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20241007133907', '2025-10-03 18:50:08.516918');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20241007133908', '2025-10-03 18:50:08.520632');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20241115160045', '2025-10-03 18:50:08.528814');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20250718085225', '2025-10-03 18:50:08.534834');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20250718104055', '2025-10-03 18:50:08.541051');
INSERT INTO public.__diesel_schema_migrations (version, run_on) VALUES ('20250917081751', '2025-10-03 18:50:08.55286');


--
-- Data for Name: bsp; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.bsp (id, account, capacity, stake, last_tick_proven, created_at, updated_at, onchain_bsp_id, merkle_root) VALUES (2, '5DCv11eqGA3JnnMnWUtbpxVZFpZZuKPoH8whZjpXebUkvAty', 536870912, 100001073741820, 0, '2025-10-03 18:50:33.615748', '2025-10-03 18:50:33.615748', '0x0000000000000000000000000000000000000000000000000000000000000002', '\xa5fb7b3e7b554b26f5f2943cb1b98649885993e9c723905a3b14b47f776a2222');
INSERT INTO public.bsp (id, account, capacity, stake, last_tick_proven, created_at, updated_at, onchain_bsp_id, merkle_root) VALUES (1, '5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg', 536870912, 100001073741820, 0, '2025-10-03 18:50:20.449767', '2025-10-03 18:50:20.449767', '0x2b83b972e63f52abc0d4146c4aee1f1ec8aa8e274d2ad1b626529446da93736c', '\xeb1291dee884f484f93526ecc95ce77ae6e982c5ca9d6bada3daba3ca2ce6409');
INSERT INTO public.bsp (id, account, capacity, stake, last_tick_proven, created_at, updated_at, onchain_bsp_id, merkle_root) VALUES (3, '5DaD86XpaVrrVK1JkmBck6pJk9eSF73c295GLnw9CC1H8sZu', 536870912, 100001073741820, 0, '2025-10-03 18:50:43.733369', '2025-10-03 18:50:43.733369', '0x0000000000000000000000000000000000000000000000000000000000000003', '\xb76c26518c6ac5fbc7a2c7d9210bd1075a5bded6fc42697303e372cef4ab6a4b');


--
-- Data for Name: bsp_file; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (2, 1);
INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (1, 2);
INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (3, 1);
INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (1, 3);
INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (3, 3);
INSERT INTO public.bsp_file (bsp_id, file_id) VALUES (3, 2);


--
-- Data for Name: bsp_multiaddress; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.bsp_multiaddress (bsp_id, multiaddress_id) VALUES (1, 1);
INSERT INTO public.bsp_multiaddress (bsp_id, multiaddress_id) VALUES (2, 4);
INSERT INTO public.bsp_multiaddress (bsp_id, multiaddress_id) VALUES (3, 5);


--
-- Data for Name: bucket; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.bucket (id, msp_id, account, onchain_bucket_id, name, collection_id, private, created_at, updated_at, merkle_root) VALUES (1, 2, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '\x8e0ec449e3a21dab7cd1276895db940843246e336e24b9a4a1a10c8d59511cde', '\x6e6f7468696e676d7563682d33', NULL, false, '2025-10-03 18:50:43.81239', '2025-10-03 18:50:43.81239', '\xb76c26518c6ac5fbc7a2c7d9210bd1075a5bded6fc42697303e372cef4ab6a4b');


--
-- Data for Name: file; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step, deletion_status, created_at, updated_at) VALUES (1, '\x20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657', '\xe592bcecc540f2363850b6895d82a310a4dd6686f066603ca2abfb77fc478b0a', 1, '\x8e0ec449e3a21dab7cd1276895db940843246e336e24b9a4a1a10c8d59511cde', '\x746573742f776861747375702e6a7067', '\x2b83b972e63f52abc0d4146c4aee1f1ec8aa8e274d2ad1b626529446da93736c', 216211, 1, NULL, '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');
INSERT INTO public.file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step, deletion_status, created_at, updated_at) VALUES (3, '\x20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657', '\x9134625b559ec290a821019522301ce7a948a8e31d8a2739d8ba1c7e55ba01b2', 1, '\x8e0ec449e3a21dab7cd1276895db940843246e336e24b9a4a1a10c8d59511cde', '\x746573742f736d696c652e6a7067', '\x535dd863026735ffe0919cc0fc3d8e5da45b9203f01fbf014dbe98005bd8d2fe', 633160, 1, NULL, '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');
INSERT INTO public.file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step, deletion_status, created_at, updated_at) VALUES (2, '\x20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657', '\x6b24ad7fd18b01148eeb4039029d5c32fa535db70ae42413e7025d74919ae5c9', 1, '\x8e0ec449e3a21dab7cd1276895db940843246e336e24b9a4a1a10c8d59511cde', '\x746573742f61646f6c706875732e6a7067', '\x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970', 416400, 1, NULL, '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');


--
-- Data for Name: file_peer_id; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.file_peer_id (file_id, peer_id) VALUES (1, 1);
INSERT INTO public.file_peer_id (file_id, peer_id) VALUES (2, 2);
INSERT INTO public.file_peer_id (file_id, peer_id) VALUES (3, 3);


--
-- Data for Name: msp; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.msp (id, account, capacity, value_prop, created_at, updated_at, onchain_msp_id) VALUES (1, '5E1rPv1M2mheg6pM57QqU7TZ6eCwbVpiYfyYkrugpBdEzDiU', 536870912, 'ValuePropositionWithId { id: 0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b, value_prop: ValueProposition { price_per_giga_unit_of_data_per_block: 104857600, commitment: BoundedVec([84, 101, 114, 109, 115, 32, 111, 102, 32, 83, 101, 114, 118, 105, 99, 101, 46, 46, 46], 1000), bucket_data_limit: 9999999, available: true } }', '2025-10-03 18:50:21.190544', '2025-10-03 18:50:21.190544', '0x0000000000000000000000000000000000000000000000000000000000000300');
INSERT INTO public.msp (id, account, capacity, value_prop, created_at, updated_at, onchain_msp_id) VALUES (2, '5CMDKyadzWu6MUwCzBB93u32Z1PPPsV8A1qAy4ydyVWuRzWR', 536870912, 'ValuePropositionWithId { id: 0x3dd8887de89f01cef28701feda1435cf0bb38e9d5cb38321a615c1a1e1d5d51b, value_prop: ValueProposition { price_per_giga_unit_of_data_per_block: 104857600, commitment: BoundedVec([84, 101, 114, 109, 115, 32, 111, 102, 32, 83, 101, 114, 118, 105, 99, 101, 46, 46, 46], 1000), bucket_data_limit: 9999999, available: true } }', '2025-10-03 18:50:21.298897', '2025-10-03 18:50:21.298897', '0x0000000000000000000000000000000000000000000000000000000000000301');


--
-- Data for Name: msp_file; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.msp_file (msp_id, file_id) VALUES (2, 1);
INSERT INTO public.msp_file (msp_id, file_id) VALUES (2, 2);
INSERT INTO public.msp_file (msp_id, file_id) VALUES (2, 3);


--
-- Data for Name: msp_multiaddress; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.msp_multiaddress (msp_id, multiaddress_id) VALUES (1, 2);
INSERT INTO public.msp_multiaddress (msp_id, multiaddress_id) VALUES (2, 3);


--
-- Data for Name: multiaddress; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.multiaddress (id, address, created_at, updated_at) VALUES (1, '\x04ac12000206768ea50326002408011220b87e930365d6c6ffc5f71f46e660ae02dc06b3d8567e0962ef7278073b997977', '2025-10-03 18:50:20.449767', '2025-10-03 18:50:20.449767');
INSERT INTO public.multiaddress (id, address, created_at, updated_at) VALUES (2, '\x04ac12000306768ea50326002408011220f79c2af6c0719176040b005d56818bfcac38cc16bb1ffb25832d1635378df0ca', '2025-10-03 18:50:21.190544', '2025-10-03 18:50:21.190544');
INSERT INTO public.multiaddress (id, address, created_at, updated_at) VALUES (3, '\x04ac12000406768ea5032600240801122006631a7d98c2644e4d2670aab0145e7427e3f07c55ac152692e07d20c000b8f8', '2025-10-03 18:50:21.298897', '2025-10-03 18:50:21.298897');
INSERT INTO public.multiaddress (id, address, created_at, updated_at) VALUES (4, '\x04ac120007067692a50326002408011220cdcc307655ee95761298dfc68841e43f6599de5654c35980a357e3daad81cd3c', '2025-10-03 18:50:33.615748', '2025-10-03 18:50:33.615748');
INSERT INTO public.multiaddress (id, address, created_at, updated_at) VALUES (5, '\x04ac120008067693a50326002408011220b61ab4f7a66c2a1d95d29ce0d3c9a1f19d4d191785f1ef42f2d07a6e519be9ae', '2025-10-03 18:50:43.733369', '2025-10-03 18:50:43.733369');


--
-- Data for Name: paymentstream; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.paymentstream (id, account, provider, total_amount_paid, last_tick_charged, charged_at_tick, rate, amount_provided) VALUES (2, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '0x0000000000000000000000000000000000000000000000000000000000000002', 0, 0, 0, NULL, 216211);
INSERT INTO public.paymentstream (id, account, provider, total_amount_paid, last_tick_charged, charged_at_tick, rate, amount_provided) VALUES (3, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '0x2b83b972e63f52abc0d4146c4aee1f1ec8aa8e274d2ad1b626529446da93736c', 0, 0, 0, NULL, 1049560);
INSERT INTO public.paymentstream (id, account, provider, total_amount_paid, last_tick_charged, charged_at_tick, rate, amount_provided) VALUES (4, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '0x0000000000000000000000000000000000000000000000000000000000000003', 0, 0, 0, NULL, 1265771);
INSERT INTO public.paymentstream (id, account, provider, total_amount_paid, last_tick_charged, charged_at_tick, rate, amount_provided) VALUES (1, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '0x0000000000000000000000000000000000000000000000000000000000000300', 691944, 27, 27, 173610, NULL);
INSERT INTO public.paymentstream (id, account, provider, total_amount_paid, last_tick_charged, charged_at_tick, rate, amount_provided) VALUES (5, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', '0x0000000000000000000000000000000000000000000000000000000000000301', 0, 0, 0, 173610, NULL);


--
-- Data for Name: peer_id; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.peer_id (id, peer, created_at, updated_at) VALUES (1, '\x313244334b6f6f574d76626874596a6268676a6f447a626e66373153467a6e4a414b42426b5347594555746e704553317939744d', '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');
INSERT INTO public.peer_id (id, peer, created_at, updated_at) VALUES (2, '\x313244334b6f6f574d76626874596a6268676a6f447a626e66373153467a6e4a414b42426b5347594555746e704553317939744d', '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');
INSERT INTO public.peer_id (id, peer, created_at, updated_at) VALUES (3, '\x313244334b6f6f574d76626874596a6268676a6f447a626e66373153467a6e4a414b42426b5347594555746e704553317939744d', '2025-10-03 18:50:44.066343', '2025-10-03 18:50:44.066343');


--
-- Data for Name: service_state; Type: TABLE DATA; Schema: public; Owner: -
--

INSERT INTO public.service_state (id, last_indexed_finalized_block, created_at, updated_at) VALUES (1, 27, '2025-10-03 18:50:08.461168+00', '2025-10-03 18:50:08.461168+00');


--
-- Name: bsp_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.bsp_id_seq', 3, true);


--
-- Name: bucket_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.bucket_id_seq', 1, true);


--
-- Name: file_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.file_id_seq', 3, true);


--
-- Name: msp_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.msp_id_seq', 2, true);


--
-- Name: multiaddress_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.multiaddress_id_seq', 5, true);


--
-- Name: paymentstream_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.paymentstream_id_seq', 5, true);


--
-- Name: peer_id_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.peer_id_id_seq', 3, true);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: bsp_file bsp_file_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_file
    ADD CONSTRAINT bsp_file_pkey PRIMARY KEY (bsp_id, file_id);


--
-- Name: bsp_multiaddress bsp_multiaddress_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_multiaddress
    ADD CONSTRAINT bsp_multiaddress_pkey PRIMARY KEY (bsp_id, multiaddress_id);


--
-- Name: bsp bsp_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp
    ADD CONSTRAINT bsp_pkey PRIMARY KEY (id);


--
-- Name: bucket bucket_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bucket
    ADD CONSTRAINT bucket_pkey PRIMARY KEY (id);


--
-- Name: file_peer_id file_peer_id_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.file_peer_id
    ADD CONSTRAINT file_peer_id_pkey PRIMARY KEY (file_id, peer_id);


--
-- Name: file file_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.file
    ADD CONSTRAINT file_pkey PRIMARY KEY (id);


--
-- Name: msp_file msp_file_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_file
    ADD CONSTRAINT msp_file_pkey PRIMARY KEY (msp_id, file_id);


--
-- Name: msp_multiaddress msp_multiaddress_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_multiaddress
    ADD CONSTRAINT msp_multiaddress_pkey PRIMARY KEY (msp_id, multiaddress_id);


--
-- Name: msp msp_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp
    ADD CONSTRAINT msp_pkey PRIMARY KEY (id);


--
-- Name: multiaddress multiaddress_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.multiaddress
    ADD CONSTRAINT multiaddress_pkey PRIMARY KEY (id);


--
-- Name: paymentstream paymentstream_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.paymentstream
    ADD CONSTRAINT paymentstream_pkey PRIMARY KEY (id);


--
-- Name: peer_id peer_id_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.peer_id
    ADD CONSTRAINT peer_id_pkey PRIMARY KEY (id);


--
-- Name: service_state service_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.service_state
    ADD CONSTRAINT service_state_pkey PRIMARY KEY (id);


--
-- Name: idx_bsp_file_bsp_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bsp_file_bsp_id ON public.bsp_file USING btree (bsp_id);


--
-- Name: idx_bsp_file_file_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bsp_file_file_id ON public.bsp_file USING btree (file_id);


--
-- Name: idx_bsp_multiaddress_bsp_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bsp_multiaddress_bsp_id ON public.bsp_multiaddress USING btree (bsp_id);


--
-- Name: idx_bucket_account; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bucket_account ON public.bucket USING btree (account);


--
-- Name: idx_bucket_blockchain_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bucket_blockchain_id ON public.bucket USING btree (onchain_bucket_id);


--
-- Name: idx_bucket_msp_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_bucket_msp_id ON public.bucket USING btree (msp_id);


--
-- Name: idx_file_bucket_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_file_bucket_id ON public.file USING btree (bucket_id);


--
-- Name: idx_file_deletion_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_file_deletion_status ON public.file USING btree (deletion_status) WHERE (deletion_status IS NOT NULL);


--
-- Name: idx_file_file_key; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_file_file_key ON public.file USING btree (file_key);


--
-- Name: idx_file_peer_id_file_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_file_peer_id_file_id ON public.file_peer_id USING btree (file_id);


--
-- Name: idx_msp_file_file_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_msp_file_file_id ON public.msp_file USING btree (file_id);


--
-- Name: idx_msp_file_msp_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_msp_file_msp_id ON public.msp_file USING btree (msp_id);


--
-- Name: idx_msp_multiaddress_msp_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_msp_multiaddress_msp_id ON public.msp_multiaddress USING btree (msp_id);


--
-- Name: idx_multiaddress_address; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_multiaddress_address ON public.multiaddress USING btree (address);


--
-- Name: idx_paymentstream_account; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_paymentstream_account ON public.paymentstream USING btree (account);


--
-- Name: idx_paymentstream_amount_provided; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_paymentstream_amount_provided ON public.paymentstream USING btree (amount_provided) WHERE (amount_provided IS NOT NULL);


--
-- Name: idx_paymentstream_provider; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_paymentstream_provider ON public.paymentstream USING btree (provider);


--
-- Name: idx_paymentstream_rate; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_paymentstream_rate ON public.paymentstream USING btree (rate) WHERE (rate IS NOT NULL);


--
-- Name: bsp_file bsp_file_bsp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_file
    ADD CONSTRAINT bsp_file_bsp_id_fkey FOREIGN KEY (bsp_id) REFERENCES public.bsp(id);


--
-- Name: bsp_file bsp_file_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_file
    ADD CONSTRAINT bsp_file_file_id_fkey FOREIGN KEY (file_id) REFERENCES public.file(id);


--
-- Name: bsp_multiaddress bsp_multiaddress_bsp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_multiaddress
    ADD CONSTRAINT bsp_multiaddress_bsp_id_fkey FOREIGN KEY (bsp_id) REFERENCES public.bsp(id) ON DELETE CASCADE;


--
-- Name: bsp_multiaddress bsp_multiaddress_multiaddress_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bsp_multiaddress
    ADD CONSTRAINT bsp_multiaddress_multiaddress_id_fkey FOREIGN KEY (multiaddress_id) REFERENCES public.multiaddress(id) ON DELETE CASCADE;


--
-- Name: bucket bucket_msp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bucket
    ADD CONSTRAINT bucket_msp_id_fkey FOREIGN KEY (msp_id) REFERENCES public.msp(id) ON DELETE CASCADE;


--
-- Name: file_peer_id file_peer_id_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.file_peer_id
    ADD CONSTRAINT file_peer_id_file_id_fkey FOREIGN KEY (file_id) REFERENCES public.file(id) ON DELETE CASCADE;


--
-- Name: file_peer_id file_peer_id_peer_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.file_peer_id
    ADD CONSTRAINT file_peer_id_peer_id_fkey FOREIGN KEY (peer_id) REFERENCES public.peer_id(id) ON DELETE CASCADE;


--
-- Name: msp_file msp_file_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_file
    ADD CONSTRAINT msp_file_file_id_fkey FOREIGN KEY (file_id) REFERENCES public.file(id);


--
-- Name: msp_file msp_file_msp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_file
    ADD CONSTRAINT msp_file_msp_id_fkey FOREIGN KEY (msp_id) REFERENCES public.msp(id);


--
-- Name: msp_multiaddress msp_multiaddress_msp_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_multiaddress
    ADD CONSTRAINT msp_multiaddress_msp_id_fkey FOREIGN KEY (msp_id) REFERENCES public.msp(id) ON DELETE CASCADE;


--
-- Name: msp_multiaddress msp_multiaddress_multiaddress_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.msp_multiaddress
    ADD CONSTRAINT msp_multiaddress_multiaddress_id_fkey FOREIGN KEY (multiaddress_id) REFERENCES public.multiaddress(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--
