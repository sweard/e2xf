import 'package:e2xf/main_viewmodel.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('log keeps only the most recent lines', () {
    final viewModel = MainViewModel();

    for (var index = 0; index < MainViewModel.maxLogLines + 5; index++) {
      viewModel.updateLog('line-$index');
    }

    final lines = viewModel.log.value.split('\n');
    expect(lines, hasLength(MainViewModel.maxLogLines));
    expect(lines.first, 'line-5');
    expect(lines.last, 'line-${MainViewModel.maxLogLines + 4}');

    viewModel.dispose();
  });
}
