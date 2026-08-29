select c.id, c.first_name, c.last_name, c.created_at,
       coalesce(array_agg(distinct bc.role order by bc.role) filter (where bc.role is not null), array[]::text[]) as "roles!"
from creators c
left join book_creators bc on bc.creator_id = c.id
where c.id = $1
group by c.id;
