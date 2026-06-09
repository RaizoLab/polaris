from main import main


def test_main_runs_without_error(capsys) -> None:
    main()
    captured = capsys.readouterr()
    assert "Polaris agent" in captured.out
