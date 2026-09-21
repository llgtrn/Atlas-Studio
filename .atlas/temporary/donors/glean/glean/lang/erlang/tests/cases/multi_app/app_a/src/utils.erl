-module(utils).
-export([format_id/1, validate/1]).

-include("shared.hrl").

-spec format_id(#request{}) -> string().
format_id(#request{id = Id}) ->
    integer_to_list(Id).

-spec validate(#request{}) -> ok | {error, invalid}.
validate(#request{payload = <<>>}) -> {error, invalid};
validate(#request{}) -> ok.

retry_delay(N) -> N * 100.
