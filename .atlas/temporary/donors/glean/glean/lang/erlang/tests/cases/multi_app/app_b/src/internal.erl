-module(internal).

-include_lib("app_a/include/shared.hrl").

run() ->
    Req = #request{id = 1, payload = <<"test">>},
    Id = utils:format_id(Req),
    Formatted = lists:flatten(io_lib:format("Request ~s", [Id])),
    Filtered = lists:filter(fun(C) -> C =/= $\s end, Formatted),
    {Filtered, ?MAX_RETRIES}.
