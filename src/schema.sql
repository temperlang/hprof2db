-- Raw (see views below)

create table header (
    id integer primary key,
    label text not null,
    id_size integer not null,
    timestamp integer not null
);

create table name (
    id integer primary key,
    name_id integer unique,
    text text not null
);

create table load_class (
    id integer primary key,
    serial integer unique,
    obj_id integer not null,
    stack_trace_serial integer not null, -- not unique,
    name_id not null
    -- foreign key(name_id) references name(name_id)
);
create index load_class_obj_id on load_class(obj_id);
create index load_class_stack_trace_serial on load_class(stack_trace_serial);

create table class (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    super_obj_id integer,
    instance_size integer not null
);
create index class_obj_id on class(obj_id);
create index class_stack_trace_serial on class(stack_trace_serial);
create index class_super_obj_id on class(super_obj_id);

create table instance (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    class_obj_id integer not null
);
create index instance_obj_id on instance(obj_id);
create index instance_class_obj_id on instance(class_obj_id);

create table type (
    id integer primary key,
    name text not null,
    size integer not null
);
create index type_name on type(name);
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
    -- TODO Remove once we list elements elsewhere.
    length integer not null
);
create index obj_array_obj_id on obj_array(obj_id);
create index obj_array_class_obj_id on obj_array(class_obj_id);

create table primitive_array (
    id integer primary key,
    obj_id integer not null,
    stack_trace_serial integer not null,
    type_id integer not null,
    -- TODO Remove once we list elements elsewhere.
    length integer not null
);
create index primitive_array_obj_id on primitive_array(obj_id);
create index primitive_array_type_id on primitive_array(type_id);

-- Views

create view ez_class as
with lclass as (
    select distinct obj_id, stack_trace_serial, name_id from load_class
)
select
    class.id,
    class.obj_id,
    class.stack_trace_serial,
    class.instance_size,
    name.name_id,
    name.text name
from class
    inner join lclass on class.obj_id = lclass.obj_id
    inner join name on lclass.name_id = name.name_id
;
