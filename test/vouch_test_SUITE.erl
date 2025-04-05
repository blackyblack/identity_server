-module(vouch_test_SUITE).
-compile([export_all, nowarn_export_all]).

-include_lib("common_test/include/ct.hrl").

all() -> [
    basic_test
].

init_per_suite(Config) ->
    application:start(identity_server),
    Config.

end_per_suite(Config) ->
    application:stop(identity_server),
    Config.

basic_test(_Config) ->
    To = <<"HdnWeX9Q94joJdFwQxNFAGB82WRiF5kbqPBKMWZFu8AJ">>,
    {ok, _} = identity:vouch(<<"HdnWeX9Q94joJdFwQxNFAGB82WRiF5kbqPBKMWZFu8AJ">>, To),
    Idt = identity:idt(To),
    0 = Idt,
    ok.
