create table if not exists content_ratings (
	id uuid,
	name text not null,
	code text not null,

	constraint pk_content_ratings_id primary key(id),
	constraint check_length_content_ratings_name check(char_length(name) <= 255),
	constraint unique_content_ratings_name unique(name),
	constraint check_length_content_ratings_code check(char_length(code) <= 255),
	constraint unique_content_ratings_code unique(code)
);

insert
	into
	content_ratings (id,
	name,
	code)
values ('019f124b-314f-73fc-8310-701df63eacea',
'EVERYONE',
'E'),
	   ('019f125c-33ef-753d-984f-08781390d2f4',
'TEEN',
'T'),
	   ('019f125c-bf63-7578-a57d-16f2f593c365',
'TEEN PLUS',
'T+'),
	   ('019f125d-2006-7a75-bcc2-4fc07e370d2d',
'MATURE',
'M')
on
conflict(code) do
update
set
	code = excluded.code;
