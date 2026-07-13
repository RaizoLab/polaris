from main import main


def test_main_status(capsys) -> None:
    code = main(["status"])
    assert code == 0
    captured = capsys.readouterr()
    payload = captured.out
    assert "polaris" in payload
    assert "hello" in payload
