-- Indices

create index name_text on name(text);

-- Name not unique because multiple class loaders.
create index class_name_id on class(name_id);
create index class_super_id on class(super_id);

create index field_class_id on field(class_id);

create index instance_class_id on instance(class_id);

create index field_value_instance_id on field_value(instance_id);
create index field_value_obj_id on field_value(obj_id);

create index obj_array_class_id on obj_array(class_id);

create index primitive_array_type_id on primitive_array(type_id);

-- Views

create view ez_class as
select c.id, c.instance_size, n.text name
from class c
join name n on c.name_id = n.id
;

create view ez_total as
select
    count(*) count,
    sum(length) * 8 + count(*) * 24 size,
    c.name
from obj_array a join ez_class c on a.class_id = c.id
group by c.id
union all
select
    count(*) count,
    sum(length) * t.size + count(*) * 24 size,
    t.name || '[]'
from primitive_array a inner join type t on a.type_id = t.id
group by t.id
union all
select
    count(*) count,
    count(*) * (c.instance_size + 16) size,
    c.name
from instance i inner join ez_class c on i.class_id = c.id
group by c.id
;
