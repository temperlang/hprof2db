import argparse
import sqlite3
import typing
from collections import Counter, defaultdict
from contextlib import closing


class Chain(typing.TypedDict):
    class_name: str
    instance_id: int
    ref: typing.Optional["Chain"]


class ChainCount(typing.TypedDict):
    class_names: tuple[str, ...]
    count: int
    target_count: int
    bad: bool


class ChainRaw(typing.TypedDict):
    class_name: bytes
    instance_id: int
    ref_id: int


class ChainTarget(typing.TypedDict):
    class_names: tuple[str, ...]
    instance_id: int


def chain_instances(chain: Chain) -> tuple[int, ...]:
    instances = []
    maybe: typing.Optional[Chain] = chain
    while maybe is not None:
        instances.append(maybe["instance_id"])
        maybe = maybe["ref"]
    return tuple(instances)


def class_chain(chain: Chain) -> tuple[str, ...]:
    class_names = []
    maybe: typing.Optional[Chain] = chain
    while maybe is not None:
        class_names.append(maybe["class_name"])
        maybe = maybe["ref"]
    return tuple(class_names)


def class_chain_target(chain: Chain) -> ChainTarget:
    class_names: list[str] = []
    maybe: typing.Optional[Chain] = chain
    last = chain
    while maybe is not None:
        last = maybe
        class_names.append(maybe["class_name"])
        maybe = maybe["ref"]
    return ChainTarget(class_names=tuple(class_names), instance_id=last["instance_id"])


def dict_factory(cursor: sqlite3.Cursor, row):
    fields = [column[0] for column in cursor.description]
    return {key: value for key, value in zip(fields, row)}


def get_refs_array(*, cursor: sqlite3.Cursor, instances: list[Chain]) -> list[ChainRaw]:
    params = ",".join("?" * len(instances))
    cursor.execute(
        f"""
        select n.text class_name, oa.id instance_id, oai.obj_id ref_id
        from obj_array_item oai
        join obj_array oa on oa.id = oai.array_id
        join class c on c.id = oa.class_id
        join name n on n.id = c.name_id
        where oai.obj_id in ({params})
        group by n.text, oa.id, oa.length, oai.obj_id
        """,
        tuple(instance["instance_id"] for instance in instances),
    )
    return cursor.fetchall()


def get_refs_field(*, cursor: sqlite3.Cursor, chains: list[Chain]) -> list[ChainRaw]:
    params = ",".join("?" * len(chains))
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
        tuple(chain["instance_id"] for chain in chains),
    )
    return cursor.fetchall()


def get_refs(
    *, cursor: sqlite3.Cursor, chains: list[Chain], instance_ids: set[int]
) -> list[Chain]:
    refs_array = get_refs_array(cursor=cursor, instances=chains)
    refs_field = get_refs_field(cursor=cursor, chains=chains)
    refs = refs_array + refs_field
    back_refs = {chain["instance_id"]: chain for chain in chains}
    results = [
        Chain(
            class_name=ref["class_name"].decode(),
            instance_id=ref["instance_id"],
            ref=back_refs[ref["ref_id"]],
        )
        for ref in refs
        # Keep only new instances.
        # TODO Except chains can still be interesting.
        # TODO How to prune boring chains?
        # if ref["instance_id"] not in instance_ids
        if ref["instance_id"] not in chain_instances(back_refs[ref["ref_id"]])
    ]
    for result in results:
        instance_ids.add(result["instance_id"])
    return results


def get_roots(cursor: sqlite3.Cursor) -> set[int]:
    cursor.execute(
        """
        select id from instance
        where id not in (
            select obj_id from field_value where obj_id is not null
            union
            select obj_id from obj_array_item where obj_id is not null
        )
        """
    )
    return set(row["id"] for row in cursor.fetchall())


def get_starters(
    *, cursor: sqlite3.Cursor, class_name: str, instance_ids: set[int]
) -> list[Chain]:
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
        result["ref"] = None
        instance_ids.add(result["instance_id"])
    return results


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


def report(
    *, depth: int, chains: list[Chain], target_min: int, total_max: int
) -> list[ChainCount]:
    chain_targets: defaultdict[tuple[str, ...], set[int]] = defaultdict(set)
    for chain in chains:
        chain_target = class_chain_target(chain)
        chain_targets[chain_target["class_names"]].add(chain_target["instance_id"])
    counter = Counter([class_chain(instance) for instance in chains])
    print(f"depth: {depth}, rows: {len(counter)}, instances: {len(chains)}")
    items = [
        ChainCount(
            class_names=item[0],
            count=item[1],
            target_count=len(chain_targets[item[0]]),
            bad=False,
        )
        for item in counter.items()
    ]
    items.sort(key=lambda item: item["target_count"], reverse=True)
    for item in items:
        if item["target_count"] < target_min or item["count"] > total_max:
            item["bad"] = True
        if item["target_count"] < target_min:
            continue
        note = " *" if item["bad"] else ""
        print(f"{item['class_names']}: {item['count']} vs {item['target_count']}{note}")
    print()
    return items


def run(*, db: str, depth_max: int, class_name: str):
    instance_ids: set[int] = set()
    target_min = 20
    total_max = 1000
    with closing(sqlite3.connect(db)) as conn:
        conn.row_factory = dict_factory
        cursor = conn.cursor()
        roots = get_roots(cursor=cursor)
        print(f"Total roots: {len(roots)}")
        print()
        chains = get_starters(
            cursor=cursor, class_name=class_name, instance_ids=instance_ids
        )
        depth = 0
        while True:
            roots_found = instance_ids & roots
            if roots_found:
                print(f"Roots found!!! -> {len(roots_found)}")
            print(f"Total instances: {len(instance_ids)}")
            counts = report(depth=depth, chains=chains, target_min=target_min, total_max=total_max)
            if depth >= depth_max:
                break
            bigs = set(count["class_names"] for count in counts if count["bad"])
            chains = [
                instance for instance in chains if class_chain(instance) not in bigs
            ]
            chains = get_refs(cursor=cursor, chains=chains, instance_ids=instance_ids)
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
