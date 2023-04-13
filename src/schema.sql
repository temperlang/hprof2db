create table name (
    id integer primary key,
    name_id integer unique,
    text text not null
);

create table class (
    id integer primary key,
    serial integer unique,
    obj_id integer,
    stack_trace_serial integer, -- not unique,
    name_id not null
    -- foreign key(name_id) references name(name_id)
);
create index class_obj_id on class(obj_id);
create index class_stack_trace_serial on class(stack_trace_serial);

create table instance (
    id integer primary key,
    class_obj_id integer
    -- foreign key(class_obj_id) references class(obj_id)
);
