import utils::*;

#[test]
fn test_alerts()
{
	assert title_case(~"123") == ~"123";
	assert title_case(~"Hmm") == ~"Hmm";
	assert title_case(~"hmm") == ~"Hmm";
}
