create table if not exists languages (
	id uuid,
	name text not null,
	code text not null,

	constraint pk_languages_id primary key(id),
	constraint check_length_languages_name check(char_length(name) <= 255),
	constraint unique_languages_name unique(name),
	constraint check_length_languages_code check(char_length(code) <= 255),
	constraint unique_languages_code unique(code)
);

insert
	into
	languages (id,
	name,
	code)
values ('019f12ac-d1fc-78f2-a4bc-d0827c0f1578',
'Japanese',
'ja'),
('019f12ac-f9f3-7b8c-ba3f-97033810d391',
'Korean',
'ko'),
('019f12ad-0c41-7022-8877-50861d4ec2a4',
'Chinese',
'zh'),
('019f12ad-1e26-7fdd-9318-b18ccd74e734',
'English',
'en'),
('019f12ad-2e9e-762d-999c-b4bc9bbdc964',
'Russian',
'ru') on
conflict(code) do
update
set
	code = excluded.code;
