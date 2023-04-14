## Example queries

List classes with instance counts and total sizes.

```sql
with lclass as (
    select distinct obj_id, stack_trace_serial, name_id from load_class
),
class_instance as (
    select
        class.obj_id,
        count(*) count,
        count(*) * (class.instance_size + 16) size,
        name.text
    from instance
        left join class on instance.class_obj_id = class.obj_id
        inner join lclass on class.obj_id = lclass.obj_id
        inner join name on lclass.name_id = name.name_id
        group by class.obj_id
)
select
    count,
    count * 1.0 / (select sum(count) from class_instance) count_frac,
    size,
    size * 1.0 / (select sum(size) from class_instance) size_frac,
    text
from class_instance
order by size desc
;
```

Find dupes in `load_class`:

```sql
with dupe as (
    select obj_id, name_id, count(*) from load_class
    group by obj_id having count(*) > 1
)
select * from class
    inner join dupe on class.obj_id = dupe.obj_id
    inner join name on dupe.name_id = name.name_id
;
```
