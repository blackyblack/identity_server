-module(vouch_handler).

-export([init/2, vouch/2]).

init(#{method := <<"POST">>, bindings := #{user := UserEncoded}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := _SignatureEncoded, <<"nonce">> := _Nonce, <<"signer">> := PublicKeyEncoded} = json:decode(JsonData),
    %RefMessage = identity_signature:message_for_signature(vouch, PublicKeyEncoded, Nonce),
    %ok = identity_signature:verify_and_consume_signature(SignatureEncoded, PublicKeyEncoded, Nonce, RefMessage),
    {ok, VouchResp} = vouch(PublicKeyEncoded, UserEncoded),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, VouchResp, State),
    {ok, Resp, Opts}.

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
