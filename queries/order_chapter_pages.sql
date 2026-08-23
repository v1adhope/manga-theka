update chapter_pages p
set sort_order = d.sort_order::smallint
from unnest($2::uuid[]) with ordinality as d(id, sort_order)
where p.release_id = $1 and p.id = d.id;
