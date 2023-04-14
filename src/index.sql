-- Indices

create unique index name_name_id on name(name_id);

create unique index load_class_serial on load_class(serial);
create index load_class_obj_id on load_class(obj_id);
create index load_class_stack_trace_serial on load_class(stack_trace_serial);

create index class_obj_id on class(obj_id);
create index class_stack_trace_serial on class(stack_trace_serial);
create index class_super_obj_id on class(super_obj_id);

create index instance_obj_id on instance(obj_id);
create index instance_class_obj_id on instance(class_obj_id);

create index type_name on type(name);

create index obj_array_obj_id on obj_array(obj_id);
create index obj_array_class_obj_id on obj_array(class_obj_id);

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
