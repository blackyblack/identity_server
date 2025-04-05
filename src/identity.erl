-module(identity).

-export([vouch/2, idt/1]).

%TODO: might require to migrate to gen_server to make IDT updates atomic

vouch(From, To) ->
    ets:insert(vouches, {{To, From}, erlang:monotonic_time()}),
    Idt = idt(To),
    % should increase 'To' IDTs, if 'From' IDT is in 'To' top 5 vouchers
    % so we need to calculate 'From' IDTs first
    {ok, json:encode(#{<<"from">> => From, <<"to">> => To, <<"idt">> => Idt})}.

idt(User) ->
    % IDT calculation requires iterating all way down the graph so it can take
    % substantial time and can be affected by graph reorganizations during
    % traversal
    _Vouchers = ets:match(vouches, {{User, '$1'}, '$2'}),
    0.
