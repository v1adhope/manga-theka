select r.id, r.chapter_id, r.version,
       l.id as language_id, l.code as language_code, l.name as language_name,
       pc.page_count as "page_count!"
from chapter_releases r
join languages l on l.id = r.language_id
cross join lateral (select count(*) as page_count
                     from chapter_pages p
                     where p.release_id = r.id and p.sort_order is not null) pc
where r.chapter_id = $1
  and pc.page_count > 0
order by r.id;
