-module(client).
-export([send_request/1, batch_send/1]).

-include_lib("app_a/include/shared.hrl").

-spec send_request(binary()) -> ok | {error, term()}.
send_request(Payload) ->
    Req = #request{id = erlang:unique_integer(), payload = Payload},
    case utils:validate(Req) of
        ok -> server:handle_request(Req);
        Err -> Err
    end.

-spec batch_send([binary()]) -> [ok | {error, term()}].
batch_send(Payloads) ->
    lists:map(fun send_request/1, Payloads).
