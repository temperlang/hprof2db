create table name (
    id integer primary key,
    text text not null
);

create table class (
    id integer primary key,
    serial integer unique,
    stack_trace_serial integer unique,
    name_id not null,
    foreign key(name_id) references name(id)
);

create table instance (
    id integer primary key,
    class_id integer,
    foreign key(class_id) references class(id)
);
