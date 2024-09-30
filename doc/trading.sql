--
-- PostgreSQL database dump
--

-- Dumped from database version 15.6 ( 15.6-0+deb12u1)
-- Dumped by pg_dump version 15.6 ( 15.6-0+deb12u1)

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

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: instrument; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.instrument (
    id bigint,
    name text,
    i_type smallint,
    state smallint,
    percentage_bands smallint,
    percentage_variation_allowed smallint,
    active smallint DEFAULT 1
);


ALTER TABLE public.instrument OWNER TO postgres;

--
-- Name: users; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.users (
    username character varying(64),
    password character varying(64),
    session_id integer,
    participant bigint,
    userttype integer
);


ALTER TABLE public.users OWNER TO postgres;

--
-- Name: instrument instrument_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.instrument
    ADD CONSTRAINT instrument_id_key UNIQUE (id);


--
-- Name: TABLE instrument; Type: ACL; Schema: public; Owner: postgres
--

GRANT SELECT ON TABLE public.instrument TO test;


--
-- Name: TABLE users; Type: ACL; Schema: public; Owner: postgres
--

GRANT SELECT ON TABLE public.users TO test;


--
-- PostgreSQL database dump complete
--

