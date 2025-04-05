-module(vouch_test_SUITE).
-export([all/0, basic_test/1]).

-include_lib("common_test/include/ct.hrl").

all() -> [
    basic_test
].

basic_test(_Config) ->
    application:start(identity_server),
    {ok, _} = vouch_handler:vouch(<<"HdnWeX9Q94joJdFwQxNFAGB82WRiF5kbqPBKMWZFu8AJ">>, <<"HdnWeX9Q94joJdFwQxNFAGB82WRiF5kbqPBKMWZFu8AJ">>),
    ok.
