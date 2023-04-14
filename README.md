## Example queries

List classes with instance counts and total sizes.

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
