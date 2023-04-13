create table class (
    id integer primary key,
    name text not null unique
);

create table instance (
    id integer primary key,
    class_id integer,
    foreign key(class_id) references class(id)
);
