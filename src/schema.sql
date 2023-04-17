create table header (
    id integer primary key,
    label text not null,
    id_size integer not null,
    timestamp integer not null
);

create table name (
    id integer primary key,
    text text not null
);

create table class (
    id integer primary key,
    name_id not null,
    super_id integer,
    instance_size integer not null
);

create table field (
    id integer primary key,
    class_id integer not null,
    name_id integer not null,
    ind integer not null,
    type_id integer not null
);

create table instance (
    id integer primary key,
    class_id integer not null
    -- Other obj ids are
    -- obj_id integer not null
);

create table field_value (
    id integer primary key,
    instance_id integer not null,
    field_id integer not null,
    -- Only one at most of these should be non null.
    -- float real,
    -- int integer,
    obj_id integer
);

create table type (
    id integer primary key,
    name text not null unique,
    size integer not null
);
-- Expected sizes when when used in arrays.
-- There might be alignment issues especially as object fields.
insert into type (id, name, size) values
    (2, 'object', 8), -- presumed reference size, any way to know???
    (4, 'boolean', 1),
    (5, 'char', 2),
    (6, 'float', 4),
    (7, 'double', 8),
    (8, 'byte', 1),
    (9, 'short', 2),
    (10, 'int', 4),
    (11, 'long', 8)
;
-- See also https://shipilev.net/blog/2014/heapdump-is-a-lie/

create table obj_array (
    id integer primary key,
    class_id integer not null,
    length integer not null
);

create table obj_array_item (
    id integer primary key,
    array_id integer not null,
    ind integer not null,
    obj_id integer not null
);

create table primitive_array (
    id integer primary key,
    type_id integer not null,
    length integer not null,
    text text -- not null if char array or utf8-compatible byte array
);

-- TODO other primitive array data
