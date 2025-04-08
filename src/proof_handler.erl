-module(proof_handler).

-export([init/2]).

% TODO: add proof tests

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := Signature, <<"nonce">> := Nonce, <<"signer">> := PublicKey, <<"idt">> := Balance} = json:decode(JsonData),
    RefMessage = identity_signature:proof_signature_message(User, Nonce, Balance),
    ok = identity_signature:verify_and_consume_signature(proof, Signature, PublicKey, Nonce, RefMessage),
    ok = identity:set_idt_by_proof(PublicKey, User, Balance),
    ProofResp = json:encode(#{<<"from">> => PublicKey, <<"to">> => User, <<"idt">> => Balance}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, ProofResp, State),
    {ok, Resp, Opts}.
