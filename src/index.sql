-- Indices

create index name_text on name(text);

-- Name not unique because multiple class loaders.
create index class_name_id on class(name_id);
create index class_super_id on class(super_id);

create index field_class_id on field(class_id);

create index instance_class_id on instance(class_id);

create index field_value_instance_id on field_value(instance_id);
create index field_value_field_id on field_value(field_id);

create index obj_array_class_id on obj_array(class_id);

create index obj_array_item_array_id on obj_array_item(array_id);

create index primitive_array_type_id on primitive_array(type_id);

-- Clean up, which presumably table scans the updated tables

-- Index what we're not already scanning.
-- We'll delete this whole table when we're done with it.
-- The point is that tracking every instance id mapping can be ram intensive.
-- So we delegate the work to the database after the fact.
create index hprof_obj_id_hprof_obj_id on hprof_obj_id(hprof_obj_id);

-- Track which didn't resolve to known objects.
update field_value
set obj_id = -1
where field_value.obj_id is not null and obj_id not in (
    select h.hprof_obj_id from hprof_obj_id h
    where field_value.obj_id = h.hprof_obj_id
)
;
-- Then link those that do.
update field_value
set obj_id = h.id
from hprof_obj_id h
where field_value.obj_id = h.hprof_obj_id
;

-- Both steps again for references from obj arrays.
update obj_array_item
set obj_id = -1
where obj_array_item.obj_id is not null and obj_id not in (
    select h.hprof_obj_id from hprof_obj_id h
    where obj_array_item.obj_id = h.hprof_obj_id
)
;
update obj_array_item
set obj_id = h.id
from hprof_obj_id h
where obj_array_item.obj_id = h.hprof_obj_id
;

-- And don't need the link anymore.
drop table hprof_obj_id;

-- Post cleanup indices, once they have useful values

create index field_value_obj_id on field_value(obj_id);

create index obj_array_item_obj_id on field_value(obj_id);

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
