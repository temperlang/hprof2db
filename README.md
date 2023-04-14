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

List fields referencing instances of particular class (replacing
`'class/name/Here'` with the class of interest):

```sql
select count(*) count, oc.name class, fn.text field from ez_class c
join instance i on c.obj_id = i.class_obj_id
join field_value v on i.obj_id = v.value_obj_id
join instance oi on v.obj_id = oi.obj_id
join field_info f on oi.class_obj_id = f.class_obj_id and v.ind = f.ind
join name fn on f.name_id = fn.name_id
join ez_class oc on oi.class_obj_id = oc.obj_id
where c.name like 'class/name/Here'
group by oc.name, fn.text
order by count(*) desc
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
