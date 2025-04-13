-module(vouch_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := Signature, <<"nonce">> := Nonce, <<"signer">> := PublicKey} = json:decode(JsonData),
    RefMessage = identity_signature:vouch_signature_message(User, Nonce),
    ok = identity_signature:verify_and_consume_signature(vouch, Signature, PublicKey, Nonce, RefMessage),
    {ok, Idt} = identity:vouch(PublicKey, User),
    VouchResp = json:encode(#{<<"from">> => PublicKey, <<"to">> => User, <<"idt">> => Idt}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, VouchResp, State),
    {ok, Resp, Opts}.
