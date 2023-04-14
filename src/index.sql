-- Indices

create unique index name_name_id on name(name_id);

create unique index load_class_serial on load_class(serial);
create index load_class_obj_id_name_id on load_class(obj_id, name_id);
create index load_class_name_id on load_class(name_id);
create index load_class_stack_trace_serial on load_class(stack_trace_serial);

create unique index class_obj_id on class(obj_id);
create index class_stack_trace_serial on class(stack_trace_serial);
create index class_super_obj_id on class(super_obj_id);

create unique index field_info_class_obj_id_index
    on field_info(class_obj_id, ind)
;
create unique index field_info_class_obj_id_name_id
    on field_info(class_obj_id, name_id)
;

create index instance_obj_id on instance(obj_id);
create index instance_class_obj_id on instance(class_obj_id);

create unique index field_value_obj_id_index on field_value(obj_id, ind);
create unique index field_value_value on field_value(value);

create unique index type_name on type(name);

create index obj_array_obj_id on obj_array(obj_id);
create index obj_array_class_obj_id on obj_array(class_obj_id);

create index primitive_array_obj_id on primitive_array(obj_id);
create index primitive_array_type_id on primitive_array(type_id);

-- Views

create view ez_class as
with lclass as (
    select distinct obj_id, name_id from load_class
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

create view ez_total as
select
    count(*) count,
    sum(length) * 8 + count(*) * 24 size,
    c.name
from obj_array a inner join ez_class c on a.class_obj_id = c.obj_id
group by c.obj_id
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
from instance i inner join ez_class c on i.class_obj_id = c.obj_id
group by c.obj_id
;
