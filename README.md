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
select c.name class, fn.text field, t.name type
from field f
join ez_class c on f.class_id = c.id
join name fn on f.name_id = fn.id
join type t on f.type_id = t.id
order by c.name, fn.text
;
```

List fields referencing instances of particular class (replacing
`'class/name/Here'` with the class of interest):

```sql
select count(*) count, oc.name class, fn.text field from ez_class c
join instance i on c.id = i.class_id
join field_value v on i.id = v.obj_id
join field f on v.field_id = f.id
join name fn on f.name_id = fn.id
join ez_class oc on v.class_id = oc.id
where c.name like 'class/name/Here'
group by oc.name, fn.text
order by count(*) desc
;
```
