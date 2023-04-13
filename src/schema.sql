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
