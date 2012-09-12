use utils::*;

#[test]
fn test_title_case()
{
	assert title_case(~"123") == ~"123";
	assert title_case(~"Hmm") == ~"Hmm";
	assert title_case(~"hmm") == ~"Hmm";
}
