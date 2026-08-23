select r.id, r.chapter_id, r.version,
       l.id as language_id, l.code as language_code, l.name as language_name,
       count(p.id) as "page_count!"
from chapter_releases r
join languages l on l.id = r.language_id
join chapter_pages p on p.release_id = r.id and p.sort_order is not null
where r.chapter_id = $1
group by r.id, l.id
order by r.id;
