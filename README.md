## Example queries

List classes with instance counts and total sizes.

```sql
with class_instance as (
    select
        c.obj_id,
        count(*) count,
        count(*) * (c.instance_size + 16) size,
        c.name
    from instance i
        left join ez_class c on i.class_obj_id = c.obj_id
        group by c.obj_id
)
select
    count,
    count * 1.0 / (select sum(count) from class_instance) count_frac,
    size,
    size * 1.0 / (select sum(size) from class_instance) size_frac,
    name
from class_instance
order by size desc
;
```

List arrays.

```sql
select
    count(*) count,
    sum(length) * 8 + count(*) * 24 size,
    name
from obj_array a inner join ez_class c on a.class_obj_id = c.obj_id
group by c.obj_id
union all
select
    count(*) count,
    sum(length) * t.size + count(*) * 24 size,
    name || '[]'
from primitive_array a inner join type t on a.type_id = t.id
group by t.id
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
