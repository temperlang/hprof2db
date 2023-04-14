## Example queries

List classes with instance counts and total sizes:

```sql
select
    count,
    count * 1.0 / (select sum(count) from ez_total) count_frac,
    size,
    size * 1.0 / (select sum(size) from ez_total) size_frac,
    name
from ez_total
order by size desc
;
```

List all class instance field names:

```sql
select cn.text class, fn.text field, t.name type
from field_info f
    join ez_class c on f.class_obj_id = c.obj_id
    join name cn on c.name_id = cn.name_id
    join name fn on f.name_id = fn.name_id
    join type t on f.type_id = t.id
order by cn.text, ind
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
