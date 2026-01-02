#include "VisualizationWindow.h"

#include "DataProvider.h"
#include "InputConfigurator.h"
#include "StatusGraph.h"
#include <QVBoxLayout>

VisualizationWindow::VisualizationWindow(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(10);

    auto* configurator = new InputConfigurator(this);
    layout->addWidget(configurator);

    auto* statusGraph = new StatusGraph(this);
    statusGraph->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    layout->addWidget(statusGraph, 1);

    auto* dataProvider = new DataProvider(this);
    connect(dataProvider, &DataProvider::dataReady, statusGraph, &StatusGraph::updateData);
    dataProvider->start();
}
