import argparse
import sqlite3
import typing
from collections import Counter
from contextlib import closing


class Classy(typing.TypedDict):
    class_name: str
    ref_class_name: typing.Optional[str]


class Count(typing.TypedDict):
    class_name: str
    ref_class_name: typing.Optional[str]
    count: int


class Ref(typing.TypedDict):
    class_name: str
    instance_id: int
    array_len: typing.Optional[int]
    ref_class_name: typing.Optional[str]


def class_pair(instance: Classy) -> tuple[str, typing.Optional[str]]:
    return (instance["class_name"], instance["ref_class_name"])


def dict_factory(cursor: sqlite3.Cursor, row):
    fields = [column[0] for column in cursor.description]
    return {key: value for key, value in zip(fields, row)}


def get_refs_array(*, cursor: sqlite3.Cursor, instances: list[Ref]) -> list[Ref]:
    params = ",".join("?" * len(instances))
    cursor.execute(
        f"""
        select n.text class_name, oa.id instance_id, oa.length array_len, oai.obj_id ref_id
        from obj_array_item oai
        join obj_array oa on oa.id = oai.array_id
        join class c on c.id = oa.class_id
        join name n on n.id = c.name_id
        where oai.obj_id in ({params})
        group by n.text, oa.id, oa.length, oai.obj_id
        """,
        tuple(instance["instance_id"] for instance in instances),
    )
    back_refs = {
        instance["instance_id"]: instance["class_name"] for instance in instances
    }
    results = cursor.fetchall()
    for result in results:
        result["class_name"] = result["class_name"].decode("utf-8")
        result["ref_class_name"] = back_refs[result.pop("ref_id")]
    return results


def get_refs_field(*, cursor: sqlite3.Cursor, instances: list[Ref]) -> list[Ref]:
    params = ",".join("?" * len(instances))
    cursor.execute(
        f"""
        select n.text class_name, i.id instance_id, fv.obj_id ref_id
        from field_value fv
        join instance i on i.id = fv.instance_id
        join class c on c.id = i.class_id
        join name n on n.id = c.name_id
        where fv.obj_id in ({params})
        group by n.text, i.id, fv.obj_id
        """,
        tuple(instance["instance_id"] for instance in instances),
    )
    back_refs = {
        instance["instance_id"]: instance["class_name"] for instance in instances
    }
    results = cursor.fetchall()
    for result in results:
        result["class_name"] = result["class_name"].decode("utf-8")
        result["array_len"] = None
        result["ref_class_name"] = back_refs[result.pop("ref_id")]
    return results


def get_refs(*, cursor: sqlite3.Cursor, instances: list[Ref]) -> list[Ref]:
    refs_array = get_refs_array(cursor=cursor, instances=instances)
    refs_field = get_refs_field(cursor=cursor, instances=instances)
    return refs_array + refs_field


def get_starters(*, cursor: sqlite3.Cursor, class_name: str) -> list[Ref]:
    cursor.execute(
        """
        select n.text class_name, i.id instance_id
        from class c
        join name n on n.id = c.name_id
        join instance i on i.class_id = c.id
        where n.text like ?
        order by n.text, i.id
        """,
        (class_name,),
    )
    results = cursor.fetchall()
    for result in results:
        result["class_name"] = result["class_name"].decode("utf-8")
        result["array_len"] = None
        result["ref_class_name"] = None
    return results


# def get_roots(cursor):
#     cursor.execute(
#         """
#         select id from instance
#         where id not in (
#             select obj_id from field_value where obj_id is not null
#             union
#             select obj_id from obj_array_item where obj_id is not null
#         )
#         """
#     )
#     return [row[0] for row in cursor.fetchall()]


# def build_reverse_refs(cursor):
#     refs = defaultdict(set)
#     cursor.execute(
#         "select instance_id, obj_id from field_value where obj_id is not null"
#     )
#     for from_id, to_id in cursor.fetchall():
#         refs[to_id].add(from_id)
#     cursor.execute(
#         "select array_id, obj_id from obj_array_item where obj_id is not null"
#     )
#     for from_id, to_id in cursor.fetchall():
#         refs[to_id].add(from_id)
#     return refs


# def trace_paths(roots, target_ids, reverse_refs):
#     paths = []
#     visited = set()
#     queue = deque([(root, [root]) for root in roots])
#     while queue:
#         current, path = queue.popleft()
#         if current in visited:
#             continue
#         visited.add(current)
#         if current in target_ids:
#             paths.append(path)
#         for parent in reverse_refs.get(current, []):
#             queue.append((parent, path + [parent]))
#     return paths


def report(*, depth: int, instances: list[Ref], limit: int) -> list[Count]:
    counter = Counter([class_pair(instance) for instance in instances])
    print(f"depth: {depth}, rows: {len(counter)}, instances: {len(instances)}")
    items = [
        Count(class_name=pair[0], ref_class_name=pair[1], count=count)
        for pair, count in counter.items()
    ]
    items.sort(key=lambda item: item["count"], reverse=True)
    for item in items:
        note = " *" if item["count"] >= limit else ""
        print(
            f"{item['class_name']} -> {item['ref_class_name']}: {item['count']}{note}"
        )
    print()
    return items


def run(*, db: str, depth_max: int, class_name: str):
    limit = 1000
    with closing(sqlite3.connect(db)) as conn:
        conn.row_factory = dict_factory
        cursor = conn.cursor()
        instances = get_starters(cursor=cursor, class_name=class_name)
        depth = 0
        while True:
            counts = report(depth=depth, instances=instances, limit=limit)
            if depth >= depth_max:
                break
            bigs = set(class_pair(count) for count in counts if count["count"] >= limit)
            instances = [
                instance for instance in instances if class_pair(instance) not in bigs
            ]
            instances = get_refs(cursor=cursor, instances=instances)
            depth += 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", required=True, help="Path to the SQLite heap database")
    parser.add_argument("--depth", required=True, type=int, help="Max depth to trace")
    parser.add_argument(
        "--class", required=True, help="Internal name pattern of the target class(es)"
    )
    args = parser.parse_args().__dict__.copy()
    args["class_name"] = args.pop("class")
    args["depth_max"] = args.pop("depth")
    run(**args)


if __name__ == "__main__":
    main()
