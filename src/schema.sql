create table header (
    id integer primary key,
    label text not null,
    id_size integer not null,
    timestamp integer not null
);

create table name (
    id integer primary key,
    name_id integer not null,
    text text not null
);

create table load_class (
    id integer primary key,
    serial integer not null,
    obj_id integer not null,
    stack_trace_serial integer not null,
    name_id not null
    -- foreign key(name_id) references name(name_id)
);

create table class (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    super_obj_id integer,
    instance_size integer not null
);

create table field_info (
    id integer primary key,
    class_obj_id integer not null,
    ind integer not null,
    name_id integer not null,
    type_id integer not null
);

create table instance (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    class_obj_id integer not null
);

create table field_value (
    id integer primary key,
    instance_obj_id integer not null,
    class_obj_id integer not null, -- because super types
    ind integer not null,
    -- Only one at most of these should be non null.
    float real,
    int integer,
    obj_id integer
);

create table type (
    id integer primary key,
    name text not null,
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
    obj_id integer not null,
    stack_trace_serial integer not null,
    class_obj_id integer not null,
    length integer not null
);

create table obj_array_item (
    id integer primary key,
    array_obj_id integer not null,
    ind integer not null,
    obj_id integer not null
);

create table primitive_array (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    type_id integer not null,
    length integer not null,
    text text -- not null if char array
);

-- TODO other primitive array data
