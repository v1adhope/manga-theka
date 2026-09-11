insert into book_creators(book_id, creator_id, role)
select $1, credit.creator_id, credit.role
from unnest($2::uuid[], $3::text[]) as credit(creator_id, role);
